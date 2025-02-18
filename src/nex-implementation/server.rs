use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use crate::prudp::packet::PRUDPPacket;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{RMCResponse, RMCResponseResult, send_response};
use crate::rmc::response::ErrorCode::Core_NotImplemented;
use crate::web::DirectionalData::Incoming;
use crate::web::WEB_DATA;

type ContainedProtocolList = Box<[Box<dyn for<'a> Fn(&'a RMCMessage, &'a Arc<SocketData>, &'a Arc<Mutex<ConnectionData>>) -> Pin<Box<dyn Future<Output = Option<RMCResponse>> + Send + 'a>> + Send + Sync>]>;

pub struct RMCProtocolServer(ContainedProtocolList);

impl RMCProtocolServer{
    pub fn new(protocols: ContainedProtocolList) -> Arc<Self>{
        Arc::new(Self(protocols))
    }

    pub async fn process_message(&self, packet: PRUDPPacket, socket: Arc<SocketData>, connection: Arc<Mutex<ConnectionData>>){
        let locked = connection.lock().await;
        let addr = locked.sock_addr.regular_socket_addr;
        drop(locked);
        let mut web = WEB_DATA.lock().await;
        web.data.push((addr, Incoming(hex::encode(&packet.payload))));
        drop(web);

        let Ok(rmc) = RMCMessage::new(&mut Cursor::new(&packet.payload)) else {
            error!("error reading rmc message");
            return;
        };

        println!("got rmc message {},{}", rmc.protocol_id, rmc.method_id);
        
        for proto in &self.0 {
            if let Some(response) = proto(&rmc, &socket, &connection).await {
                if matches!(response.response_result, RMCResponseResult::Error {..}){
                    error!("an rmc error occurred")
                }
                let mut locked = connection.lock().await;
                send_response(&packet, &socket, &mut locked, response).await;
                drop(locked);
                return;
            }
        }

        error!("tried to send message to unimplemented protocol {} with method id {}", rmc.protocol_id, rmc.method_id);
        let mut locked = connection.lock().await;
        send_response(&packet, &socket, &mut locked, RMCResponse{
            protocol_id: rmc.protocol_id as u8,
            response_result: RMCResponseResult::Error {
                call_id: rmc.call_id,
                error_code: Core_NotImplemented
            }
        }).await;

    }
}