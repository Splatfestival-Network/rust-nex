use std::io::{Read, Seek};
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use super::{string, Result};

#[derive(Debug)]
pub struct Any{
    pub name: String,
    pub data: Vec<u8>
}

pub fn read(reader: &mut (impl Read + Seek)) -> Result<Any>{
    let name = string::read(reader)?;

    // also length ?
    let len2: u32 = reader.read_struct(IS_BIG_ENDIAN)?;
    let length: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

    let mut data = vec![0; length as usize];

    reader.read_exact(&mut data)?;

    Ok(
        Any{
            name,
            data
        }
    )
}