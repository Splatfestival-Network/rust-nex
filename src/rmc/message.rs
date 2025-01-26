use std::io;
use std::io::{Read, Seek};
use log::error;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::response::{ErrorCode, RMCResponseResult};

#[derive(Debug)]
pub struct RMCMessage{
    pub protocol_id: u16,
    pub call_id: u32,
    pub method_id: u32,

    pub rest_of_data: Vec<u8>
}

impl RMCMessage{
    pub fn new(stream: &mut (impl Seek + Read)) -> io::Result<Self>{
        let size: u32 = stream.read_struct(IS_BIG_ENDIAN)?;

        let mut header_size = 1 + 4 + 4;

        let protocol_id: u8 = stream.read_struct(IS_BIG_ENDIAN)?;
        let protocol_id= protocol_id & (!0x80);

        let protocol_id: u16 = match protocol_id{
            0x7F => {
                header_size += 2;
                stream.read_struct(IS_BIG_ENDIAN)?
            },
            _ => protocol_id as u16
        };

        let call_id = stream.read_struct(IS_BIG_ENDIAN)?;
        let method_id = stream.read_struct(IS_BIG_ENDIAN)?;

        let mut rest_of_data = Vec::new();

        stream.read_to_end(&mut rest_of_data)?;

        if header_size + rest_of_data.len() != size as usize {
            error!("received incorrect rmc packet: expected size {} but found {}", size, header_size + rest_of_data.len());
        }



        //stream.
        Ok(Self{
            protocol_id,
            method_id,
            call_id,
            rest_of_data
        })
    }

    pub fn error_result_with_code(&self, error_code: ErrorCode) -> RMCResponseResult{
        RMCResponseResult::Error {
            call_id: self.call_id,
            error_code
        }
    }
}