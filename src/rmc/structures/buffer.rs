use std::io::{Read, Write};
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::RmcSerialize;

impl<'a> RmcSerialize for &'a [u8]{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        let u32_size = self.len() as u32;
        writer.write(bytemuck::bytes_of(&u32_size))?;
        writer.write(self)?;

        Ok(())
    }

    /// DO NOT USE (also maybe split off the serialize and deserialize functions at some point)
    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        panic!("cannot deserialize to a u8 slice reference (use this ONLY for writing)")
    }
}

impl<'a> RmcSerialize for Vec<u8>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let len: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

        let mut data = vec![0; len as usize];

        reader.read_exact(&mut data)?;

        Ok(data)
    }
}

impl<'a> RmcSerialize for Box<[u8]>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Vec::deserialize(reader).map(|v| v.into_boxed_slice())
    }
}