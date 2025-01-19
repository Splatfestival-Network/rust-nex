use std::net::Ipv4Addr;
use rc4::Rc4;

pub trait AuthModule{
    fn get_auth_key(addr: Ipv4Addr) -> [u8; 32];
}
/*
struct AuthServerAuthModule;

impl AuthModule for AuthServerAuthModule{
    fn get_auth_key(addr: Ipv4Addr) -> rc4 {

    }
}*/