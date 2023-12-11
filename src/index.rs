use rkyv::{Archive, Deserialize, Serialize};

// very small pseudo hash 3 bytes to 2 bytes
pub fn hash_3_to_2(byte1: u8, byte2: u8, byte3: u8) -> u16 {
    // Use the first byte and XOR with the high bits of the second byte
    let high = u16::from(byte1) ^ (u16::from(byte2) << 4);
    
    // Use the low bits of the second byte and XOR with the third byte
    let low = (u16::from(byte2) >> 4) ^ u16::from(byte3);
    
    (high << 8) | low
}

// write compact bool vector to index file
// index element
#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct Index {
    pub offset:u64,
    pub compress_size:u32, // it's enough by u32, but use u64 for padding
    pub original_size:u32,
    pub hash: [u64; 65536 / 64]
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct ListofIndex {
    pub n:u32,
    pub indexies:Vec<Index>
}

pub fn fill_index(index:&mut Index,v:&Vec<bool>) {
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
