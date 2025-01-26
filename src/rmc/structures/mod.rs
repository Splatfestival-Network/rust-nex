use std::io;
use std::io::{Read, Seek, Write};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use md5::digest::impl_oid_carrier;
use thiserror::Error;

//ideas for the future: make a proc macro library which allows generation of struct reads

#[derive(Error, Debug)]
pub enum Error{
    #[error("Io Error: {0}")]
    Io(#[from] io::Error),
    #[error("UTF8 conversion Error: {0}")]
    Utf8(#[from] FromUtf8Error)
}

type Result<T> = std::result::Result<T, Error>;

pub mod string;
pub mod any;

pub trait RmcSerialize: Sized{
    fn serialize(&self, writer: &mut dyn Write) -> Result<()>;
    fn deserialize(reader: &mut dyn Read) -> Result<Self>;
}