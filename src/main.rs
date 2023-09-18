#[allow(unused_imports)]
use log::{info, warn, Level};
use simple_logger::SimpleLogger;

// args
use clap::{arg, command, value_parser};
use std::path::PathBuf;

// file operation for create index
use std::fs;
use std::io::{Read, Write, self, Error};

// file operation for search
extern crate nix;
use nix::fcntl;
use nix::sys::stat;
use nix::libc;

// index
use std::mem;
use rkyv::{Archive, Deserialize, Serialize};
use rkyv::ser::{Serializer, serializers::AllocSerializer};

// very small pseudo hash 3 bytes to 2 bytes
fn hash_3_to_2(byte1: u8, byte2: u8, byte3: u8) -> u16 {
    // Use the first byte and XOR with the high bits of the second byte
    let high = u16::from(byte1) ^ (u16::from(byte2) << 4);
    
    // Use the low bits of the second byte and XOR with the third byte
    let low = (u16::from(byte2) >> 4) ^ u16::from(byte3);
    
    (high << 8) | low
}

// write compact bool vector to index file
// index element
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
struct Index {
    offset:u64,
    size:u64, // it's enough by u32, but use u64 for padding
    hash: [u64; 65536 / 64]
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
struct ListofIndex {
    n:usize,
    indexies:Vec<Index>
}


fn fill_index(index:&mut Index,v:&Vec<bool>) {
    let hash_bytes = unsafe {
        std::slice::from_raw_parts_mut(
            index.hash.as_mut_ptr() as *mut u8,
            (65536/64) * std::mem::size_of::<u64>(),
        )
    };

    for i in 0..(65536/8) {
        let mut u:u8 = 0;
        for j in i*8..i*8+8 {
            if v[j] { u|=1; }
            u <<= 1;
        }
        hash_bytes[i] = u;
    };
    log::debug!("{:x?}",hash_bytes);
}

// create index

fn create_index(file:&mut fs::File, index:&mut fs::File,chunk_size:usize) -> std::io::Result<()> {
    let mut read_buff: Vec<u8> = vec![0;chunk_size];
    let mut bits:Vec::<bool> = vec![true;65536];
    let mut remains = chunk_size;
    let mut read_count: usize = 0;
    let mut nread :usize = 0;

    let mut offset:u64 = 0;
    let mut indexies :ListofIndex = ListofIndex { n: 0, indexies: Vec::new() };
    
    loop {
        for item in bits.iter_mut() {
            *item = false;
        }
        remains = chunk_size;
        read_count = 0;

        // fill buffer of chunk size
        while remains>0 {
            nread = file.read(&mut read_buff[read_count..])?;
            if nread==0 { break };
            remains -= nread;
            read_count += nread;
        };
        let iter = read_buff[0..read_count].windows(3);
        log::debug!("nread={}",nread);
        iter.for_each(|s| {
            bits[hash_3_to_2(s[0], s[1], s[2]) as usize]=true;
            //bits[(s[0] as usize*s[1] as usize*s[2] as usize) % 65536]=true
        });

        let mut ielm = Index{offset:offset, size:read_count as u64, hash:[0u64;8192/8]};
        log::debug!("offset={}, size={}",ielm.offset,ielm.size);
        fill_index(&mut ielm, &bits);
        indexies.indexies.push(ielm);
        indexies.n += 1;
        offset += read_count as u64;

        
        if nread==0 { break };
    };

    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&indexies).unwrap();
    let bytes = serializer.into_serializer().into_inner();
    index.write_all(&bytes)
}


// generate query vector
fn fill_query(query_string:String) -> Vec<u64> {
    let mut bits:Vec::<bool> = vec![false;65536];
    let query_bytes = query_string.into_bytes();
    let iter = query_bytes.windows(3);

    iter.for_each(|s| {
        bits[hash_3_to_2(s[0], s[1], s[2]) as usize]=true;
    });

    let mut bits_compact: Vec<u8> = Vec::new();

    for i in 0..(65536/8) {
        let mut u:u8 = 0;
        for j in i*8..i*8+8 {
            if bits[j] { u|=1; }
            u <<= 1;
        }
        bits_compact.push(u);
    };
    log::debug!("bits_compact_len = {}, u64 size = {}", bits_compact.len(), bits_compact.len()/8);
    log::debug!("bits_compact = {:x?}",bits_compact);

    let mut u64_vec = vec![0;bits_compact.len() / 8];
    for chunk in bits_compact.chunks_exact(8) {
        log::debug!("chunk = {:x?}",chunk);
        let arr_ref: &[u8; 8] = &chunk.try_into().expect("Failed to convert slice into array");
        let u64_val: u64 = unsafe { mem::transmute_copy(arr_ref) };
        u64_vec.push(u64_val);
    };

    u64_vec

}

// check matching
fn match_query(query:&Vec<u64>,index:&[u64]) -> bool {
    for (q,i) in query.iter().zip(index.iter()) {
        if *q==0 { continue; };
        if (*q & *i)==*q { continue;};
        return false;
    };
    true
}

// query
fn query(file_fd:std::os::fd::RawFd, index: &mut fs::File, query_string:String,chunk_size:usize) -> std::io::Result<()> {
    // read index
    let mut index_buff_u8: Vec<u8> = Vec::new();
    let read_to_end_ret =index.read_to_end(&mut index_buff_u8);
    log::info!("read_to_end_ret: {:?}", read_to_end_ret);

    let archived = unsafe { rkyv::archived_root::<ListofIndex>(&index_buff_u8[..]) };
    let deserialized: ListofIndex = archived.deserialize(&mut rkyv::Infallible).unwrap();
    log::debug!("deserialize len = {}", deserialized.n);

    let query = fill_query(query_string);
    log::debug!("fill_query = {:?}",query);

    let num_of_index = deserialized.n;
    
    let mut file_buf:Vec<u8> = vec![0;chunk_size];
    let mut ith_index = 0;
    for ielm in deserialized.indexies.iter() {
        log::debug!("ielm offset = {} size={}",ielm.offset,ielm.size);

        if match_query(&query,&ielm.hash) {
            log::info!("matched!");
            let mut nread = 0;
            let mut remain = ielm.size as usize;
            let mut read_pos:usize = 0;
            let mut offset = ielm.offset as i64;

            while remain>0 {
                log::debug!("remain={}, offset={}, read_pos={}, nread={}",remain,offset,read_pos,nread);
                unsafe {
                    nread = libc::pread(file_fd,file_buf[read_pos..read_pos+remain].as_mut_ptr() as *mut libc::c_void,remain,offset);
                };
                match nread {
                    -1 => {
                        let err = nix::errno::errno();
                        return Err(Error::from_raw_os_error(err))
                    },
                    0 => { break },
                    _ => {
                        remain -= nread as usize;
                        offset += nread as i64;
                        read_pos += nread as usize;
                    }
                };
            };

            io::stdout().write_all(&file_buf)?;
            // extract_chunk(file,buff);
            // do_query();
        };

        ith_index+=1;
        log::info!("ith index match = {}",ith_index);
        if ith_index >= num_of_index {     
            log::info!("ielm loop finished");   
            break 
        };
    };
    log::info!("search() finished");

    Ok(())
}

fn main() -> std::io::Result<()> {
    let matches = command!()
        .arg(arg!(-f --file <FILE>)
            .help("target file")
            .value_parser(value_parser!(PathBuf))
            .required(true))
        .arg(arg!(-i --index <INDEX>)
            .help("index file")
            .value_parser(value_parser!(PathBuf))
            .default_value("index.dat"))
        .arg(arg!(-c --chunk <CHUNK>)
            .help("chunk size should be 2^N")
            .value_parser(["4M","8M","16M"])
            .default_value("4M"))
        .arg(arg!(-l --log <LOG>)
            .default_value("info")
            .value_parser(["debug","info","warn"])
            .help("Set the logging level. Options: [error, warn, info, debug, trace]"))
        .arg(arg!(-q --query <QUERY>)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .required_unless_present("create")
            .help("query string"))
        .arg(arg!(-C --create)
            .required_unless_present("query")
            .help("create index"))
    .get_matches();

    let chunk_size = match matches.get_one::<String>("chunk").unwrap().as_str() {
        "4M" => 4*1024*1024, // default 
        "8M" => 8*1024*1024,
        "16M" => 16*1024*1024,
        &_ => todo!(),
    };

    // Get the logging level from the command line, or default to 'info'
    let log_level = match matches.get_one::<String>("log").unwrap().as_str() {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        _ => Level::Info,
    };
    
    // logging
    SimpleLogger::new().with_level(log_level.to_level_filter()).init().unwrap();
    log::debug!("finish argument parsing");

    /*
        This is an expermental code. Do not care of chunks divide lines
     */
    let query_string = matches.get_one::<String>("query");
    let file_path = matches.get_one::<PathBuf>("file").unwrap();
    let index_path = matches.get_one::<PathBuf>("index").unwrap();
    
    log::debug!("query = {:?}",query_string);
    match query_string {
        Some(q) =>  
        {
            let mut index = fs::File::open(index_path)?;
            let file_fd = fcntl::open(file_path, nix::fcntl::OFlag::O_RDONLY , stat::Mode::empty())?;
            query(file_fd,&mut index, q.clone(),chunk_size)?;
        },
        None => {
            let mut index = fs::OpenOptions::new().write(true).create(true).open(index_path)?;
            let mut file = fs::File::open(file_path)?;
            create_index(&mut file,&mut index,chunk_size)?;
        } 
    };

    Ok(())
}

