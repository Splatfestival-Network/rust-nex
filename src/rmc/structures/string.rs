use std::ffi::CString;
use std::io::{Read, Seek};
use log::error;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use super::Result;

pub fn read(reader: &mut (impl Read + Seek)) -> Result<String>{
    let len: u16 = reader.read_struct(IS_BIG_ENDIAN)?;
    let mut data = vec![0; len as usize - 1];
    reader.read_exact(&mut data)?;

    let null: u8 = reader.read_struct(IS_BIG_ENDIAN)?;
    if null != 0{
        error!("unable to find null terminator... continuing anyways");
    }

    Ok(String::from_utf8(data)?)
}