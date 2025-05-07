use std::io::{Read, Write};
use bytemuck::bytes_of;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::RmcSerialize;



// this is also for implementing `Buffer` this is tecnically not the same as its handled internaly 
// probably but as it has the same mapping it doesn't matter and simplifies things
impl<T: RmcSerialize> RmcSerialize for Vec<T>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        let u32_len = self.len() as u32;

        writer.write_all(bytes_of(&u32_len))?;
        for e in self{
            e.serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let len: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

        let mut vec = Vec::with_capacity(len as usize);

        for _ in 0..len{
            vec.push(T::deserialize(reader)?);
        }

        Ok(vec)
    }
}
