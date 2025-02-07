use std::io::{Read, Write};
use bytemuck::bytes_of;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::RmcSerialize;

impl RmcSerialize for u8{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u16{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u32{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for i64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for f64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for bool{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        match self{
            true => writer.write_all(&[1])?,
            false => writer.write_all(&[0])?,
        }
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(u8::deserialize(reader)? != 0)
    }
}


impl<T: RmcSerialize, U: RmcSerialize> RmcSerialize for (T, U){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;

        Ok((first, second))
    }
}