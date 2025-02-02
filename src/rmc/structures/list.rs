use std::io::{Read, Write};
use bytemuck::bytes_of;
use crate::rmc::structures::RmcSerialize;


impl<T: RmcSerialize> RmcSerialize for Vec<T>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        let u32_len = self.len();

        writer.write_all(bytes_of(&u32_len))?;
        for e in self{
            e.serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        todo!()
    }
}
