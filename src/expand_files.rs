use std::process;
use std::fs::File;
use std::io::{Read, Write};
use crate::index::ListofIndex;
use lz4_flex::block::{decompress_into, get_maximum_output_size};
use rkyv::Deserialize;

// create index
pub fn expand_file(source:&mut File, target:&mut File,  index:&mut File,chunk_size:usize) -> std::io::Result<()> {
    let mut read_buff: Vec<u8> = vec![0;chunk_size]; // source reading chunk buffer
    let mut expand_buffer:Vec<u8> = vec![0;get_maximum_output_size(chunk_size)];

    // read out index and evaluate as ListofIndex
    let mut index_buff_u8: Vec<u8> = Vec::new();
    let read_to_end_ret =index.read_to_end(&mut index_buff_u8);
    log::info!("index: {:?}, read_to_end_ret: {:?}, len of index_buff_u8: {:?}", index, read_to_end_ret,index_buff_u8.len());
    log::debug!("index_buff_u8: {:x?}",index_buff_u8);
    let archived = unsafe { rkyv::archived_root::<ListofIndex>(&index_buff_u8[..]) };
    let deserialized: ListofIndex = archived.deserialize(&mut rkyv::Infallible).unwrap();
    log::debug!("deserialize len = {}", deserialized.n);
    
    for idx in deserialized.indexies {
        log::info!("idx: {:?}",idx);
        source.read_exact(&mut read_buff[0..idx.compress_size as usize])?;
        match decompress_into(&read_buff[0..idx.compress_size as usize], &mut expand_buffer) {
            Err(e) =>    { log::error!("an error at file:{} line:{} ,msg:{}",file!(),line!(), e); process::exit(1); },
            _ => {}
        };
        target.write_all(&expand_buffer[0..(idx.original_size as usize)])?;
    };
    Ok(())
}