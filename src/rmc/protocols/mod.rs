#![allow(async_fn_in_trait)]

pub mod auth;
pub mod secure;
pub mod notifications;
pub mod matchmake;
pub mod matchmake_extension;
pub mod nat_traversal;
pub mod matchmake_ext;
pub mod ranking;

use crate::prudp::socket::{ExternalConnection, SendingConnection};
use crate::rmc::message::RMCMessage;
use crate::rmc::protocols::RemoteCallError::ConnectionBroke;
use crate::rmc::response::{ErrorCode, RMCResponse, RMCResponseResult};
use crate::rmc::structures;
use crate::rmc::structures::connection_data::ConnectionData;
use crate::rmc::structures::matchmake::AutoMatchmakeParam;
use crate::rmc::structures::{Error, RmcSerialize};
use async_trait::async_trait;
use chrono::TimeDelta;
use log::{error, info};
use macros::method_id;
use macros::{rmc_proto, rmc_struct};
use paste::paste;
use std::collections::HashMap;
use std::io::Cursor;
use std::ops::{Add, Deref};
use std::sync::{Arc, Condvar};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, Notify};
use tokio::time::{sleep_until, Instant};
use crate::result::ResultExtension;

#[derive(Error, Debug)]
pub enum RemoteCallError {
    #[error("Call to remote timed out whilest waiting on response.")]
    Timeout,
    #[error("A server side rmc error occurred: {0:?}")]
    ServerError(ErrorCode),
    #[error("Connection broke")]
    ConnectionBroke,
    #[error("Error reading response data: {0}")]
    InvalidResponse(#[from] structures::Error),
}

pub struct RmcConnection(pub SendingConnection, pub RmcResponseReceiver);

pub struct RmcResponseReceiver(Arc<Notify>, Arc<Mutex<HashMap<u32, RMCResponse>>>);

impl RmcConnection {
    pub async fn make_raw_call<T: RmcSerialize>(
        &self,
        message: &RMCMessage,
    ) -> Result<T, RemoteCallError> {
        self.make_raw_call_no_response(message).await?;

        let data = self.1.get_response_data(message.call_id).await?;

        let out = <T as RmcSerialize>::deserialize(&mut Cursor::new(data))?;

        Ok(out)
    }

    pub async fn make_raw_call_no_response(
        &self,
        message: &RMCMessage,
    ) -> Result<(), RemoteCallError> {
        let message_data = message.to_data();

        self.0.send(message_data).await.ok_or(ConnectionBroke)?;

        Ok(())
    }
}

impl RmcResponseReceiver {
    // returns none if timed out
    pub async fn get_response_data(&self, call_id: u32) -> Result<Vec<u8>, RemoteCallError> {
        let mut end_wait_time = Instant::now();
        end_wait_time += Duration::from_secs(5);

        let sleep_fut = sleep_until(end_wait_time);
        tokio::pin!(sleep_fut);

        let mut sleep_manual_unlock_fut = Instant::now();
        sleep_manual_unlock_fut += Duration::from_secs(4);

        let sleep_manual_unlock_fut = sleep_until(sleep_manual_unlock_fut);
        tokio::pin!(sleep_manual_unlock_fut);

        loop {
            let mut locked = self.1.lock().await;

            if let Some(v) = locked.remove(&call_id) {
                match v.response_result{
                    RMCResponseResult::Success {
                        data,
                        ..
                    } => return Ok(data),
                    RMCResponseResult::Error {
                        error_code,
                        ..
                    } => return Err(RemoteCallError::ServerError(error_code))
                }
            }

            drop(locked);

            let notif_fut = self.0.notified();

            tokio::select! {
                _ = &mut sleep_manual_unlock_fut => {
                    continue;
                }
                _ = &mut sleep_fut => {
                    return Err(RemoteCallError::Timeout);
                }
                _ = notif_fut => {
                    continue;
                }
            }
        }
    }
}

pub trait HasRmcConnection {
    fn get_connection(&self) -> &RmcConnection;
}

pub trait RemoteObject {
    fn new(conn: RmcConnection) -> Self;
}

impl RemoteObject for () {
    fn new(_: RmcConnection) -> Self {}
}

pub trait RmcCallable {
    //type Remote: RemoteObject;
    fn rmc_call(
        &self,
        responder: &SendingConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: Vec<u8>,
    ) -> impl std::future::Future<Output = ()> + Send;
}

#[macro_export]
macro_rules! define_rmc_proto {
    (proto $name:ident{
        $($protocol:path),*
    }) => {
        paste::paste!{
            pub trait [<Local $name>]: std::any::Any $( + [<Raw $protocol>] + $protocol)* {
                async fn rmc_call(&self, remote_response_connection: &crate::prudp::socket::SendingConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: Vec<u8>){
                    match protocol_id{
                        $(
                            [<Raw $protocol Info>]::PROTOCOL_ID => <Self as [<Raw $protocol>]>::rmc_call_proto(self, remote_response_connection, method_id, call_id, rest).await,
                        )*
                        v => log::error!("invalid protocol called on rmc object {}", v)
                    }
                }
            }

            pub struct [<Remote $name>](crate::rmc::protocols::RmcConnection);

            impl crate::rmc::protocols::RemoteInstantiatable for [<Remote $name>]{
                fn new(conn: crate::rmc::protocols::RmcConnection) -> Self{
                    Self(conn)
                }
            }

            impl crate::rmc::protocols::HasRmcConnection for [<Remote $name>]{
                fn get_connection(&self) -> &crate::rmc::protocols::RmcConnection{
                    &self.0
                }
            }

            $(
            impl [<Remote $protocol>] for [<Remote $name>]{}
            )*
        }
    };
}

/// This is a special case to allow unit to represent the fact that no object is represented.
impl RmcCallable for () {
    async fn rmc_call(
        &self,
        remote_response_connection: &crate::prudp::socket::SendingConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: Vec<u8>,
    ) {
        //todo: maybe reply with not implemented(?)
    }
}

pub trait RemoteInstantiatable{
    fn new(conn: RmcConnection) -> Self;
}

pub struct OnlyRemote<T: RemoteInstantiatable>(T);

impl<T: RemoteInstantiatable> Deref for OnlyRemote<T>{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: RemoteInstantiatable> OnlyRemote<T>{
    pub fn new(conn: RmcConnection) -> Self{
        Self(T::new(conn))
    }
}

impl<T: RemoteInstantiatable> RmcCallable for OnlyRemote<T>{
    fn rmc_call(&self, responder: &SendingConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: Vec<u8>) -> impl std::future::Future<Output = ()> + Send {
        async{}
    }
}

async fn handle_incoming<T: RmcCallable + Send + Sync + 'static>(
    mut connection: ExternalConnection,
    remote: Arc<T>,
    notify: Arc<Notify>,
    incoming: Arc<Mutex<HashMap<u32, RMCResponse>>>,
) {
    let sending_conn = connection.duplicate_sender();

    while let Some(v) = connection.recv().await{
        let Some(proto_id) = v.get(4) else {
            error!("received too small rmc message.");
            error!("ending rmc gateway.");
            return
        };

        if (proto_id & 0x80) == 0{
            let Some(response) = RMCResponse::new(&mut Cursor::new(v)).display_err_or_some() else {
                error!("ending rmc gateway.");
                return
            };

            info!("got rmc response");

            let mut locked = incoming.lock().await;

            locked.insert(response.get_call_id(), response);
            notify.notify_waiters();
        } else {
            let Some(message) = RMCMessage::new(&mut Cursor::new(v)).display_err_or_some() else {
                error!("ending rmc gateway.");
                return
            };

            let RMCMessage{
                protocol_id,
                method_id,
                call_id,
                rest_of_data
            } = message;

            info!("RMC REQUEST: Proto: {}; Method: {};", protocol_id, method_id);

            remote.rmc_call(&sending_conn, protocol_id, method_id, call_id, rest_of_data).await; 

            
        }
    }
    
    info!("rmc disconnected")
}

pub fn new_rmc_gateway_connection<T: RmcCallable + Sync + Send + 'static,F>(conn: ExternalConnection, create_internal: F) -> Arc<T>
where
    F: FnOnce(RmcConnection) -> Arc<T>,
{
    let notify = Arc::new(Notify::new());
    let incoming: Arc<Mutex<HashMap<u32, RMCResponse>>> = Default::default();

    let response_recv = RmcResponseReceiver(notify.clone(), incoming.clone());

    let sending_conn = conn.duplicate_sender();

    let rmc_conn = RmcConnection(sending_conn, response_recv);

    let exposed_object = (create_internal)(rmc_conn);

    {
        let exposed_object = exposed_object.clone();
        tokio::spawn(async move {
            handle_incoming(
                conn,
                exposed_object,
                notify,
                incoming
            ).await;
        });
    }

    exposed_object
}
