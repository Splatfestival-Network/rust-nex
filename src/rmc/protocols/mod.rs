use macros::method_id;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Condvar};
use std::time::Duration;
use async_trait::async_trait;
use chrono::TimeDelta;
use macros::{rmc_proto, rmc_struct};
use paste::paste;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep_until, Instant};
use crate::prudp::socket::{ExternalConnection, SendingConnection};
use crate::rmc::structures::connection_data::ConnectionData;
use crate::rmc::structures::matchmake::AutoMatchmakeParam;

pub struct RmcConnection(pub SendingConnection, pub RmcResponseReceiver);

pub struct RmcResponseReceiver(Notify, Mutex<HashMap<(u16, u32), Vec<u8>>>);

impl RmcResponseReceiver{
    // returns none if timed out
    pub async fn get_response_data(&self, proto: u16, method: u32) -> Option<Vec<u8>>{
        let mut end_wait_time = Instant::now();
        end_wait_time += Duration::from_secs(5);

        let sleep_fut = sleep_until(end_wait_time);
        tokio::pin!(sleep_fut);

        loop {
            let mut locked = self.1.lock().await;

            if let Some(v) = locked.remove(&(proto, method)){
                return Some(v);
            }

            let notif_fut = self.0.notified();

            drop(locked);

            tokio::select! {
                _ = &mut sleep_fut => {
                    return None;
                }
                _ = notif_fut => {
                    continue;
                }
            }
        }
    }
}

pub trait RemoteObject{
    fn new(conn: RmcConnection) -> Self;
}

impl RemoteObject for (){
    fn new(_: RmcConnection) -> Self {}
}

pub trait RmcCallable{
    //type Remote: RemoteObject;
    //fn new_callable(remote: Self::Remote);
    async fn rmc_call(&self, protocol_id: u16, method_id: u32, rest: Vec<u8>);
}




macro_rules! define_rmc_proto {
    (proto $name:ident{
        $($protocol:path),*
    }) => {
        paste!{
            trait [<Local $name>]: std::any::Any $( + [<Raw $protocol>] + $protocol)* {
                async fn rmc_call(&self, protocol_id: u16, method_id: u32, rest: Vec<u8>){
                    match protocol_id{
                        $(
                            [<Raw $protocol Info>]::PROTOCOL_ID => <Self as [<Raw $protocol>]>::rmc_call_proto(self, method_id, rest).await,
                        )*
                        v => log::error!("invalid protocol called on rmc object {}", v)
                    }
                }
            }
        }
    };
}

trait RawNotif{
    async fn rmc_call_proto(&self, method_id: u32, rest: Vec<u8>){

    }
}
trait Notif{

}

struct RawNotifInfo;
impl RawNotifInfo{
    const PROTOCOL_ID: u16 = 10;
}

pub trait ImplementRemoteCalls{}

#[rmc_proto(1, NoReturn)]
pub trait Another{
    #[method_id(1)]
    async fn test(&self, thing: AutoMatchmakeParam);
}

define_rmc_proto!{
    proto TestProto{
        Notif,
        Another
    }
}



#[rmc_struct(TestProto)]
struct TestProtoImplementor{
    
}



impl Notif for TestProtoImplementor{

}

impl RawNotif for TestProtoImplementor{

}

impl Another for TestProtoImplementor{
    async fn test(&self, thing: AutoMatchmakeParam) {

    }
}

impl ImplementRemoteCalls for TestProtoImplementor{}