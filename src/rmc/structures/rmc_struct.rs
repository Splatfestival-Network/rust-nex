use std::io::Write;
use bytemuck::bytes_of;
use crate::rmc::structures::Result;

#[repr(C, packed)]
struct StructureHeader{
    version: u8,
    length: u32
}

pub fn write_struct(mut writer: &mut dyn Write, version: u8, pred: impl Fn(&mut Vec<u8>)) -> Result<()> {
    writer.write_all(&[version])?;

    let mut scratch_space: Vec<u8> = Vec::new();

    (pred)(&mut scratch_space);

    let u32_size= scratch_space.len() as u32;

    writer.write_all(bytes_of(&u32_size))?;
    writer.write_all(&scratch_space)?;

    Ok(())
}