use macros::RmcSerialize;
use crate::kerberos::KerberosDateTime;
use crate::prudp::sockaddr::PRUDPSockAddr;

#[derive(Debug, RmcSerialize)]
#[rmc_struct(0)]
pub struct ConnectionInitData{
    pub prudpsock_addr: PRUDPSockAddr,
    pub pid: u32,
    
}

