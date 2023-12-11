use std::process;
use std::fs;
use std::io::{Read, Write};
use crate::index::{Index, ListofIndex, hash_3_to_2, fill_index};
use lz4_flex::block::{compress_into,get_maximum_output_size};

// create index
pub fn create_files(source:&mut fs::File, target:&mut fs::File,  index:&mut fs::File,chunk_size:usize) -> std::io::Result<()> {
    let mut read_buff: Vec<u8> = vec![0;chunk_size]; // source reading chunk buffer
    let mut bits:Vec::<bool> = vec![true;65536]; // hash hold bit vector
    let mut nread :usize = 0;

    let mut indexies :ListofIndex = ListofIndex { n: 0, indexies: Vec::new() };

    let mut compressed_buffer:Vec<u8> = vec![0;get_maximum_output_size(chunk_size)];
    let mut compress_offset:u64 = 0;
    
    loop {
        for item in bits.iter_mut() {
            *item = false;
        }
        let mut remains = chunk_size;
        let mut read_count = 0;

        // fill buffer of chunk size
        while remains>0 {
            nread = source.read(&mut read_buff[read_count..])?;
            if nread==0 { break };
            remains -= nread;
            read_count += nread;
        };

        // generate triple-bytes hashes from chunk
        // hashes are hold in memory
        let iter = read_buff[0..read_count].windows(3);
        log::debug!("nread={}",nread);
        iter.for_each(|s| {
            bits[hash_3_to_2(s[0], s[1], s[2]) as usize]=true;
            //bits[(s[0] as usize*s[1] as usize*s[2] as usize) % 65536]=true
        });

        // compress read_buff and write it to target
        let compress_count = match compress_into(&read_buff[0..read_count as usize],&mut compressed_buffer) {
            Ok(s) => { s }
            Err(e) => { log::error!("an error at file:{} line:{} ,msg:{}",file!(),line!(), e); process::exit(1); }
        };
        target.write_all(&compressed_buffer[0..compress_count])?;
    
        
        // add an index block
        let mut ielm = Index{offset:compress_offset, compress_size:compress_count as u32, original_size:(read_count as u32),hash:[0u64;8192/8]};
        log::debug!("offset={}, compress size={}",ielm.offset,ielm.compress_size);
        fill_index(&mut ielm, &bits);
        indexies.indexies.push(ielm);
        indexies.n += 1;

        compress_offset += compress_count as u64;


        if nread==0 { break };
    };

    log::debug!("indexies: {:?}",indexies);
    // serialize and flush hashes to index file
    //let mut serializer = AllocSerializer::<0>::default();
    //serializer.serialize_value(&indexies).unwrap();
    //let bytes = serializer.into_serializer().into_inner();
    let bytes = rkyv::to_bytes::<_, 256>(&indexies).unwrap();
    log::info!("bytes len: {:?}", bytes.len());
    log::debug!("bytes: {:x?}", bytes);
    let archived = unsafe { rkyv::archived_root::<ListofIndex>(&bytes[..]) };
    
    //use rkyv::Deserialize;
    //let deserialized: ListofIndex = archived.deserialize(&mut rkyv::Infallible).unwrap();
    //assert_eq!(deserialized,indexies);
    index.write_all(&bytes)
}