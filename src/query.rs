use std::process;
use std::fs::File;
use std::io::{Read, Write, self, Error};
// file operation for search
extern crate nix;
#[allow(unused_imports)]
use nix::fcntl;
#[allow(unused_imports)]
use nix::sys::stat;
use nix::libc;
use std::mem;
use rkyv::Deserialize;

use crate::index::{ListofIndex,hash_3_to_2};
use lz4_flex::block::{decompress_into, get_maximum_output_size};


// generate query vector
fn fill_query(query_string:&String) -> Vec<u64> {
    let mut bits:Vec::<bool> = vec![false;65536];
    let query_bytes = query_string.as_bytes();
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

    let mut u64_vec = Vec::new();
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
    log::debug!("len query={}, len index={}", query.len(), index.len());
    for (q,i) in query.iter().zip(index.iter()) {
        if *q==0 { continue; };
        if (*q & *i)==*q { continue;};
        return false;
    };
    true
}

// query
pub fn query(file_fd:std::os::fd::RawFd, index: &mut File, query_string:&String,chunk_size:usize) -> std::io::Result<()> {
    // read index
    let mut index_buff_u8: Vec<u8> = Vec::new();
    let mut read_buff: Vec<u8> = vec![0;chunk_size]; // source reading chunk buffer
    let mut expand_buffer:Vec<u8> = vec![0;get_maximum_output_size(chunk_size)];
    let read_to_end_ret =index.read_to_end(&mut index_buff_u8);
    log::info!("read_to_end_ret: {:?}", read_to_end_ret);

    let archived = unsafe { rkyv::archived_root::<ListofIndex>(&index_buff_u8[..]) };
    log::debug!("unsafe rkyv finished");
    let deserialized: ListofIndex = archived.deserialize(&mut rkyv::Infallible).unwrap();
    log::debug!("deserialize len = {}", deserialized.n);

    let query = fill_query(query_string);
    log::debug!("fill_query = {:x?}",query);

    let num_of_index = deserialized.n;
    
    let mut file_buf:Vec<u8> = vec![0;chunk_size];
    let mut expand_buf:Vec<u8> = vec![0;get_maximum_output_size(chunk_size)];
    let mut ith_index = 0;
    for ielm in deserialized.indexies.iter() {
        log::debug!("ielm offset = {}",ielm.offset);
        log::debug!("ielm.hash = {:x?}",ielm.hash);
        log::info!("query string = {}",&query_string);
        log::debug!("query = {:x?}",query);
        
        if match_query(&query,&ielm.hash) {
            log::info!("matched!");
            let mut nread = 0;
            let mut remain = ielm.compress_size as usize;
            let mut rcount: usize = 0;
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
                        rcount += nread as usize;
                        offset += nread as i64;
                        read_pos += nread as usize;
                    }
                };
            };

            match decompress_into(&file_buf[0..rcount], &mut expand_buf) {
                Err(e) =>    { log::error!("an error at file:{} line:{} ,msg:{}",file!(),line!(), e); process::exit(1); },
                _ => {
                    // write string query code and output to STDOUT
                    //target.write_all(&expand_buf[0..(ielm.original_size as usize)])?;
                    //io::stdout().write_all(&file_buf)?;
                },
            };
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