use std::io::{Read, Write};
use bytemuck::bytes_of;
use crate::kerberos::KerberosDateTime;
use crate::rmc::structures::{rmc_struct, RmcSerialize};

pub struct ConnectionData<'a>{
    pub station_url: &'a str,
    pub special_protocols: Vec<u8>,
    pub special_station_url: &'a str,
    pub date_time: KerberosDateTime
}

impl<'a> RmcSerialize for ConnectionData<'a>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        rmc_struct::write_struct(writer, 1, |v|{
            self.station_url.serialize(v).expect("unable to write station url");
            self.special_protocols.serialize(v).expect("unable to write special protocols");
            self.special_station_url.serialize(v).expect("unable to write special station url");
            v.write_all(bytes_of(&self.date_time)).expect("unable to write date time");

            Ok(())
        })
    }

    fn deserialize(_reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        todo!()
    }
}

