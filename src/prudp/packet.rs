use std::fmt::{Debug, Formatter};
use std::hint::unreachable_unchecked;
use std::io;
use std::io::{Cursor, ErrorKind, Read, Seek};
use std::net::SocketAddrV4;
use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use v_byte_macros::{EnumTryInto, SwapEndian};
use crate::endianness::{IS_BIG_ENDIAN, IS_LITTLE_ENDIAN, ReadExtensions};
use crate::prudp::sockaddr::PRUDPSockAddr;

#[derive(Error, Debug)]
pub enum Error{
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("invalid magic {0:#06x}")]
    InvalidMagic(u16),
    #[error("invalid version {0}")]
    InvalidVersion(u8),
    #[error("invalid option id {0}")]
    InvalidOptionId(u8),
    #[error("option size {size} doesnt match expected option for given option id {id}")]
    InvalidOptionSize{
        id: u8,
        size: u8
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[repr(transparent)]
#[derive(Copy, Clone, Pod, Zeroable, SwapEndian)]
pub struct TypesFlags(u16);

impl TypesFlags{
    pub fn get_types(self) -> u8 {
        (self.0 & 0x000F) as u8
    }

    pub fn get_flags(self) -> u16 {
        (self.0 & 0xFFF0) >> 4
    }

    pub fn types(self, val: u8) -> Self {
        Self((self.0 & 0xFFF0) | (val as u16 & 0x000F))
    }

    pub fn flags(self, val: u16) -> Self {
        Self((self.0 & 0x000F) | ((val << 4) & 0xFFF0) )
    }
}

impl Debug for TypesFlags{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_types();
        let port_number = self.get_flags();
        write!(f, "TypesFlags{{ types: {}, flags: {} }}", stream_type, port_number)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Pod, Zeroable, SwapEndian)]
pub struct VirtualPort(u8);

impl VirtualPort{
    pub fn get_stream_type(self) -> u8 {
        (self.0 & 0xF0) >> 4
    }

    pub fn get_port_number(self) -> u8 {
        (self.0 & 0x0F)
    }

    pub fn stream_type(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0xF0) | masked_val)
    }

    pub fn port_number(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0x0F) | (masked_val << 4))
    }
}

impl Debug for VirtualPort{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_stream_type();
        let port_number = self.get_port_number();
        write!(f, "VirtualPort{{ stream_type: {}, port_number: {} }}", stream_type, port_number)
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, SwapEndian)]
pub struct PRUDPHeader{
    magic: [u8; 2],
    version: u8,
    pub packet_specific_size: u8,
    pub payload_size: u16,
    pub source_port: VirtualPort,
    pub destination_port: VirtualPort,
    pub types_and_flags: TypesFlags,
    pub session_id: u8,
    pub substream_id: u8,
    pub sequence_id: u16,
}
#[repr(u16)]
#[derive(EnumTryInto)]
enum PacketSpecificData{
    E = 0x10
}

#[derive(Debug)]
pub struct PRUDPPacket{
    pub header: PRUDPHeader,
    pub payload: Vec<u8>
}

#[derive(Copy, Clone, Debug)]
// Invariant: can only contain 0, 1, 2, 3 or 4
struct OptionId(u8);

impl OptionId{
    fn new(val: u8) -> Result<Self>{
        // Invariant is upheld because we only create the object if it doesn't violate the invariant
        match val {
            0 | 1 | 2 | 3 | 4 => Ok(Self(val)),
            _ => Err(Error::InvalidOptionId(val))
        }
    }

    fn option_type_size(self) -> u8{
        match self.0{
            0 => 4,
            1 => 16,
            2 => 1,
            3 => 2,
            4 => 1,
            // Getting here would mean that the invariant has been violated, thus this isnt my 
            // problem lmao
            _ => unsafe { unreachable_unchecked() }
        }
    }
}

impl Into<u8> for OptionId{
    fn into(self) -> u8 {
        self.0
    }
}

impl PRUDPPacket{
    pub fn new(reader: &mut (impl Read + Seek)) -> Result<Self>{
       let header: PRUDPHeader = reader.read_struct(IS_BIG_ENDIAN)?;

        if header.magic[0] != 0xEA || 
            header.magic[1] != 0xD0{
            return Err(Error::InvalidMagic(u16::from_be_bytes(header.magic)));
        }

        if header.version != 1{
            return Err(Error::InvalidVersion(header.version))
        }

        //discard it for now
        let _: [u8; 16] = reader.read_struct(IS_BIG_ENDIAN)?;

        assert_eq!(reader.stream_position().ok(), Some(14+16));

        let mut packet_specific_buffer = vec![0u8; header.packet_specific_size as usize];

        reader.read_exact(&mut packet_specific_buffer)?;



        //no clue whats up with options but they are broken
        /*let mut packet_specific_data_cursor = Cursor::new(&packet_specific_buffer);

        
        loop {
            let Ok(option_id): io::Result<u8> = packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN) else {
                break
            };

            let Ok(value_size): io::Result<u8> = packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN) else {
                break
            };

            if value_size == 0 {
                // skip it if its 0 and dont check?
                continue;
            }
            
            let option_id: OptionId = OptionId::new(option_id)?;
            
            if option_id.option_type_size() != value_size{
                return Err(Error::InvalidOptionSize {
                    size: value_size,
                    id: option_id.0
                })
            }

            let mut option_data = vec![0u8,value_size];
            if packet_specific_data_cursor.read_exact(&mut option_data).is_err(){
                break;
            }
        }*/


        let mut payload = vec![0u8; header.payload_size as usize];

        reader.read_exact(&mut payload)?;

        Ok(Self{
            header,
            payload
        })
    }

    pub fn source_sockaddr(&self,socket_addr_v4: SocketAddrV4) -> PRUDPSockAddr{
        PRUDPSockAddr{
            regular_socket_addr: socket_addr_v4,
            virtual_port: self.header.source_port
        }
    }
}

#[cfg(test)]
mod test{
    use super::{PRUDPHeader};
    #[test]
    fn size_test(){
        assert_eq!(size_of::<PRUDPHeader>(), 14);
    }
}