use std::io::Read;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::Result;
pub fn read(reader: &mut impl Read) -> Result<Vec<u8>>{
    let size: u16 = reader.read_struct(IS_BIG_ENDIAN)?;

    let mut vec = vec![0; size as usize];

    reader.read_exact(&mut vec)?;

    Ok(vec)
}