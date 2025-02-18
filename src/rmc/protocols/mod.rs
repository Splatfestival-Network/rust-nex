use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use crate::prudp::socket::ExternalConnection;
use crate::rmc::structures::connection_data::ConnectionData;

pub trait RmcCallable{
    fn rmc_call(protocol_id: u8, method_id: u8, rest: Vec<u8>);
}

struct LocalRmcObjectWrapper<T: RmcCallable>(T);
impl<T: RmcCallable> LocalRmcObjectWrapper<T>{
    pub fn new(object: T, conn: ExternalConnection) -> Self{
        unimplemented!()
    }
}

