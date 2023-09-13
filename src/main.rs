use log::{info, warn, Level};
use simple_logger::SimpleLogger;

// args
use clap::{arg, command, value_parser, Error};
use std::path::PathBuf;

// file
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

// index
use std::slice;
use std::mem;

// very small pseudo hash 3 bytes to 2 bytes
fn hash_3_to_2(byte1: u8, byte2: u8, byte3: u8) -> u16 {
    // Use the first byte and XOR with the high bits of the second byte
    let high = u16::from(byte1) ^ (u16::from(byte2) << 4);
    
    // Use the low bits of the second byte and XOR with the third byte
    let low = (u16::from(byte2) >> 4) ^ u16::from(byte3);
    
    (high << 8) | low
}

// write compact bool vector to index file
fn write_chunked_index(f:&mut File, v:&Vec<bool>) -> std::io::Result<()> {
    let mut bits_compact: Vec<u8> = Vec::new();

    for i in 0..(65536/8) {
        let mut u:u8 = 0;
        for j in i*8..i*8+8 {
            if v[j] { u|=1; }
            u <<= 1;
        }
        bits_compact.push(u);
    }
    log::debug!("size = {}",bits_compact.len());
    log::debug!("{:x?}",bits_compact);
    f.write_all(&bits_compact)
}

// create index
fn create_index(file:&mut File, index:&mut File,chunk_size:usize) -> std::io::Result<()> {
    let mut read_buff: Vec<u8> = vec![0;chunk_size];
    let mut bits:Vec::<bool> = vec![true;65536];
    let mut remains = chunk_size;
    let mut read_count: usize = 0;
    let mut nread :usize = 0;

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
        let mut iter = read_buff[0..read_count].windows(3);
        log::debug!("nread={}",nread);
        iter.for_each(|s| {
            bits[hash_3_to_2(s[0], s[1], s[2]) as usize]=true;
            //bits[(s[0] as usize*s[1] as usize*s[2] as usize) % 65536]=true
        });

        write_chunked_index(index,&bits)?;

        if nread==0 { break };
    }
    Ok(())
}


// generate query vector
fn fill_query(query_string:String) -> Vec<u64> {
    let mut bits:Vec::<bool> = vec![false;65536];
    let query_bytes = query_string.into_bytes();
    let mut iter = query_bytes.windows(3);

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
    log::debug!("bits_compact = {:x?}",bits_compact);

    let mut u64_vec = Vec::with_capacity(bits_compact.len() / 8);
    for chunk in bits_compact.chunks_exact(8) {
        log::debug!("chunk = {:x?}",chunk);
        let arr_ref: &[u8; 8] = &chunk.try_into().expect("Failed to convert slice into array");
        let u64_val: u64 = unsafe { mem::transmute_copy(arr_ref) };
        u64_vec.push(u64_val);
    }

    u64_vec

}

// check matching
fn match_query(query:Vec<u64>,index:Vec<u64>) -> bool {
    for (q,i) in query.iter().zip(index.iter()) {
        if *q==0 { continue; };
        if (*q & *i)==*q { continue;};
        return false;
    };
    true
}

// search
fn search(file:&mut File, index:&mut File, chunk_size:usize, query_string:String) -> std::io::Result<()> {
    // read index
    let mut index_buff_u8: Vec<u8> = Vec::new();
    index.read_to_end(&mut index_buff_u8)?;
    let u64_slice:&[u64] = unsafe {
        mem::transmute(&index_buff_u8[..])
    };

    let mut index_point:usize = 0;
    const u64_slice_size:usize  = 8*1024/(64/8);

    let query = fill_query(query_string);
    log::debug!("fill_query = {:x?}",query);

    if match_query(query,u64_slice[index_point..index_point+u64_slice_size].to_vec()) {
        log::debug!("matched!")
        // extract_chunk(file,buff);
        // do_query();
    };

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
    let mut file = File::open(file_path)?;
    log::debug!("query = {:?}",query_string);
    match query_string {
        Some(q) =>  
        {
            let mut index =  OpenOptions::new().read(true).open(index_path)?;
            search(&mut file,&mut index, chunk_size, q.clone())?; 
        }
        ,None => {
            let mut index = OpenOptions::new().write(true).create(true).open(index_path)?;
            create_index(&mut file,&mut index,chunk_size)?;
        } 
    };

    Ok(())
}

