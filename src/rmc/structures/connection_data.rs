use std::io::{Read, Write};
use bytemuck::bytes_of;
use macros::RmcSerialize;
use crate::kerberos::KerberosDateTime;
use crate::rmc::structures::{rmc_struct, RmcSerialize};

#[derive(Debug, RmcSerialize)]
#[rmc_struct(1)]
pub struct ConnectionData{
    pub station_url: String,
    pub special_protocols: Vec<u8>,
    pub special_station_url: String,
    pub date_time: KerberosDateTime
}

