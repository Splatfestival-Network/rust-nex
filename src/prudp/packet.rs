// no clue why this produces a warning where `#[repr(u16)]` is below,
// the thing is says to do also breaks the code, so we just
// force the compiler to shut up here
#![allow(unused_parens)]

use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{Cursor, Read, Seek, Write};
use std::net::SocketAddrV4;
use bytemuck::{Pod, Zeroable};
use hmac::{Hmac, Mac};
use log::{error, trace, warn};
use md5::{Md5, Digest};
use thiserror::Error;
use v_byte_macros::{EnumTryInto, SwapEndian};
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::prudp::packet::flags::ACK;
use crate::prudp::packet::PacketOption::{ConnectionSignature, FragmentId, InitialSequenceId, MaximumSubstreamId, SupportedFunctions};
use crate::prudp::sockaddr::PRUDPSockAddr;

type Md5Hmac = Hmac<Md5>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("invalid magic {0:#06x}")]
    InvalidMagic(u16),
    #[error("invalid version {0}")]
    InvalidVersion(u8),
    #[error("invalid option id {0}")]
    InvalidOptionId(u8),
    #[error("option size {size} doesnt match expected option for given option id {id}")]
    InvalidOptionSize {
        id: u8,
        size: u8,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[repr(transparent)]
#[derive(Copy, Clone, Pod, Zeroable, SwapEndian, Default)]
pub struct TypesFlags(u16);

impl TypesFlags {
    pub const fn get_types(self) -> u8 {
        (self.0 & 0x000F) as u8
    }

    pub const fn get_flags(self) -> u16 {
        (self.0 & 0xFFF0) >> 4
    }

    pub const fn types(self, val: u8) -> Self {
        Self((self.0 & 0xFFF0) | (val as u16 & 0x000F))
    }

    pub const fn flags(self, val: u16) -> Self {
        Self((self.0 & 0x000F) | ((val << 4) & 0xFFF0))
    }

    pub const fn set_flag(&mut self, val: u16){
        self.0 |= (val & 0xFFF) << 4;
    }

    pub const fn set_types(&mut self, val: u8){
        self.0 |= val as u16 & 0x0F;
    }
}

pub mod flags {
    pub const ACK: u16 = 0x001;
    pub const RELIABLE: u16 = 0x002;
    pub const NEED_ACK: u16 = 0x004;
    pub const HAS_SIZE: u16 = 0x008;
    pub const MULTI_ACK: u16 = 0x200;
}

pub mod types {
    pub const SYN: u8 = 0x0;
    pub const CONNECT: u8 = 0x1;
    pub const DATA: u8 = 0x2;
    pub const DISCONNECT: u8 = 0x3;
    pub const PING: u8 = 0x4;
    /// no idea what user is supposed to mean
    pub const USER: u8 = 0x5;
}

impl Debug for TypesFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_types();
        let port_number = self.get_flags();
        write!(f, "TypesFlags{{ types: {}, flags: {} }}", stream_type, port_number)
    }
}

#[repr(transparent)]
#[derive(PartialEq, Eq, Copy, Clone, Pod, Zeroable, SwapEndian, Hash)]
pub struct VirtualPort(pub(crate) u8);

impl VirtualPort {
    #[inline]
    pub const fn get_stream_type(self) -> u8 {
        (self.0 & 0xF0) >> 4
    }

    #[inline]
    pub const fn get_port_number(self) -> u8 {
        self.0 & 0x0F
    }

    #[inline]
    pub fn stream_type(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0x0F) | (masked_val << 4))
    }

    #[inline]
    pub fn port_number(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0xF0) | masked_val)
    }

    #[inline]
    pub fn new(port: u8, stream_type: u8) -> Self {
        Self(0).stream_type(stream_type).port_number(port)
    }
}

impl Debug for VirtualPort {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_stream_type();
        let port_number = self.get_port_number();
        write!(f, "VirtualPort{{ stream_type: {}, port_number: {} }}", stream_type, port_number)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, SwapEndian)]
pub struct PRUDPHeader {
    pub magic: [u8; 2],
    pub version: u8,
    pub packet_specific_size: u8,
    pub payload_size: u16,
    pub source_port: VirtualPort,
    pub destination_port: VirtualPort,
    pub types_and_flags: TypesFlags,
    pub session_id: u8,
    pub substream_id: u8,
    pub sequence_id: u16,
}

impl Default for PRUDPHeader{
    fn default() -> Self {
        Self{
            magic: [0xEA, 0xD0],
            version: 1,
            session_id: 0,
            source_port: VirtualPort(0),
            sequence_id: 0,
            payload_size: 0,
            destination_port: VirtualPort(0),
            types_and_flags: TypesFlags(0),
            packet_specific_size: 0,
            substream_id: 0
        }
    }
}


#[derive(EnumTryInto)]

#[repr(u16)]
enum PacketSpecificData {
    E = 0x10
}

#[derive(Debug, Clone)]
pub enum PacketOption{
    SupportedFunctions(u32),
    ConnectionSignature([u8; 16]),
    FragmentId(u8),
    InitialSequenceId(u16),
    MaximumSubstreamId(u8)
}

impl PacketOption{
    fn from(option_id: OptionId, option_data: &[u8]) -> io::Result<Self>{

        let mut data_cursor = Cursor::new(option_data);
        let val = match option_id.into(){
            0 => SupportedFunctions(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            1 => ConnectionSignature(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            2 => FragmentId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            3 => InitialSequenceId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            4 => MaximumSubstreamId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            _ => unreachable!()
        };

        Ok(val)
    }

    fn write_to_stream(&self, stream: &mut impl Write) -> io::Result<()> {
        match self {
            SupportedFunctions(v) => {
                stream.write_all(&[0, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            ConnectionSignature(v) => {
                stream.write_all(&[1, size_of_val(v) as u8])?;
                stream.write_all(v)?;
            }
            FragmentId(v) => {
                stream.write_all(&[2, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            InitialSequenceId(v) => {
                stream.write_all(&[3, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            MaximumSubstreamId(v) => {
                stream.write_all(&[4, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn write_size(&self) -> u8 {
        match self {
            SupportedFunctions(_) => 2 + 4,
            ConnectionSignature(_) => 2 + 16,
            FragmentId(_) => 2 + 1,
            InitialSequenceId(_) => 2 + 2,
            MaximumSubstreamId(_) => 2 + 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PRUDPPacket {
    pub header: PRUDPHeader,
    pub packet_signature: [u8; 16],
    pub payload: Vec<u8>,
    pub options: Vec<PacketOption>,
}

#[derive(Copy, Clone, Debug)]
// Invariant: can only contain 0, 1, 2, 3 or 4
struct OptionId(u8);

impl OptionId {
    fn new(val: u8) -> Result<Self> {
        // Invariant is upheld because we only create the object if it doesn't violate the invariant
        match val {
            0 | 1 | 2 | 3 | 4 => Ok(Self(val)),
            _ => Err(Error::InvalidOptionId(val))
        }
    }

    fn option_type_size(self) -> u8 {
        match self.0 {
            0 => 4,
            1 => 16,
            2 => 1,
            3 => 2,
            4 => 1,
            _ => unreachable!()
        }
    }
}

impl Into<u8> for OptionId {
    fn into(self) -> u8 {
        self.0
    }
}

impl PRUDPPacket {
    pub fn new(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let header: PRUDPHeader = reader.read_struct(IS_BIG_ENDIAN)?;

        if header.magic[0] != 0xEA ||
            header.magic[1] != 0xD0 {
            return Err(Error::InvalidMagic(u16::from_be_bytes(header.magic)));
        }

        if header.version != 1 {
            return Err(Error::InvalidVersion(header.version));
        }


        let packet_signature: [u8; 16] = reader.read_struct(IS_BIG_ENDIAN)?;

        assert_eq!(reader.stream_position().ok(), Some(14 + 16));

        let mut packet_specific_buffer = vec![0u8; header.packet_specific_size as usize];

        reader.read_exact(&mut packet_specific_buffer)?;


        //no clue whats up with options but they are broken
        let mut packet_specific_data_cursor = Cursor::new(&packet_specific_buffer);

        let mut options = Vec::new();

        loop {
            let Ok(option_id): io::Result<u8> = packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN) else {
                break
            };

            let Ok(value_size): io::Result<u8> = packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN) else {
                break
            };

            if value_size == 0 {
                // skip it if its 0 and dont check?
                warn!("reading packets options might be going wrong");
                continue;
            }

            let option_id: OptionId = OptionId::new(option_id)?;

            if option_id.option_type_size() != value_size {
                error!("invalid packet options");
                return Err(Error::InvalidOptionSize {
                    size: value_size,
                    id: option_id.0,
                });
            }

            let mut option_data = vec![0u8; value_size as usize];
            if packet_specific_data_cursor.read_exact(&mut option_data[..]).is_err() {
                error!("unable to read options");
                break;
            }

            options.push(PacketOption::from(option_id, &option_data)?);
        }

        trace!("reading payload");
        let mut payload = vec![0u8; header.payload_size as usize];

        reader.read_exact(&mut payload)?;



        Ok(Self {
            header,
            packet_signature,
            payload,
            options,
        })
    }

    pub fn base_acknowledgement_packet(&self) -> Self{
        let base = self.base_response_packet();

        let mut flags = self.header.types_and_flags.flags(0);

        flags.set_flag(ACK);

        let options = self.options
            .iter()
            .filter(|o| matches!(o, FragmentId(_)))
            .cloned()
            .collect();

        Self{
            header: PRUDPHeader{
                types_and_flags: flags,
                sequence_id: self.header.sequence_id,
                substream_id: self.header.substream_id,
                ..base.header
            },
            options,
            ..base
        }
    }

    pub fn source_sockaddr(&self, socket_addr_v4: SocketAddrV4) -> PRUDPSockAddr {
        PRUDPSockAddr {
            regular_socket_addr: socket_addr_v4,
            virtual_port: self.header.source_port,
        }
    }

    fn generate_options_bytes(&self) -> Vec<u8>{
        let mut vec = Vec::new();

        for option in &self.options{
            option.write_to_stream(&mut vec).expect("vec should always automatically be able to extend");
        }

        vec
    }

    pub fn calculate_signature_value(&self, access_key: &str, session_key: Option<[u8; 32]>, connection_signature: Option<[u8; 16]>) -> [u8; 16]{
        let access_key_bytes = access_key.as_bytes();
        let access_key_sum: u32 = access_key_bytes.iter().map(|v| *v as u32).sum();
        let access_key_sum_bytes: [u8; 4] = access_key_sum.to_le_bytes();

        let header_data: [u8; 8] = bytemuck::bytes_of(&self.header)[0x6..].try_into().unwrap();

        let option_bytes = self.generate_options_bytes();

        let mut md5 = md5::Md5::default();

        md5.update(access_key_bytes);
        let key = md5.finalize();

        let mut hmac = Md5Hmac::new_from_slice(&key).expect("fuck");

        hmac.write(&header_data).expect("error during hmac calculation");
        if let Some(session_key) = session_key {
            hmac.write(&session_key).expect("error during hmac calculation");
        }
        hmac.write(&access_key_sum_bytes).expect("error during hmac calculation");
        if let Some(connection_signature) = connection_signature {
            hmac.write(&connection_signature).expect("error during hmac calculation");
        }

        hmac.write(&option_bytes).expect("error during hmac calculation");

        hmac.write_all(&self.payload).expect("error during hmac calculation");

        hmac.finalize().into_bytes()[0..16].try_into().expect("invalid hmac size")
    }

    pub fn calculate_and_assign_signature(&mut self, access_key: &str, session_key: Option<[u8; 32]>, connection_signature: Option<[u8; 16]>){
        self.packet_signature = self.calculate_signature_value(access_key, session_key, connection_signature);
    }

    pub fn set_sizes(&mut self){
        self.header.packet_specific_size = self.options.iter().map(|o| o.write_size()).sum();
        self.header.payload_size = self.payload.len() as u16;
    }

    pub fn  base_response_packet(&self) -> Self {
        Self {
            header: PRUDPHeader {
                magic: [0xEA, 0xD0],
                types_and_flags: TypesFlags(0),
                destination_port: self.header.source_port,
                source_port: self.header.destination_port,
                payload_size: 0,
                version: 1,
                packet_specific_size: 0,
                sequence_id: 0,
                session_id: 0,
                substream_id: 0,

            },
            packet_signature: [0; 16],
            payload: Default::default(),
            options: Default::default()
        }
    }

    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()>{
        writer.write_all(bytemuck::bytes_of(&self.header))?;
        writer.write_all(&self.packet_signature)?;

        for option in &self.options{
            option.write_to_stream(writer)?;
        }

        writer.write_all(&self.payload)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{OptionId, PacketOption, PRUDPHeader, TypesFlags, VirtualPort};
    #[test]
    fn size_test() {
        assert_eq!(size_of::<PRUDPHeader>(), 14);
    }

    #[test]
    fn test_options(){
        let packet_types = [0,1,2,3,4];

        for p_type in packet_types{
            let option_id = OptionId::new(p_type).unwrap();

            let buf = vec![0; option_id.option_type_size() as usize];

            let opt = PacketOption::from(option_id, &buf).unwrap();
            {
                let mut write_buf = vec![];

                opt.write_to_stream(&mut write_buf).unwrap();

                assert_eq!(write_buf.len() as u8, opt.write_size())
            }
        }


    }

    #[test]
    fn header_read(){
        let header = PRUDPHeader{
            version: 0,
            destination_port: VirtualPort(0),
            substream_id: 0,
            types_and_flags: TypesFlags(0),
            session_id: 0,
            packet_specific_size: 0,
            payload_size: 0,
            sequence_id: 0,
            magic: [0xEA,0xD0],
            source_port: VirtualPort(0)
        };

        let bytes = bytemuck::bytes_of(&header);

        let bytes = &bytes[0x6..];

        let header_data: [u8; 8] = bytes.try_into().unwrap();
    }
}