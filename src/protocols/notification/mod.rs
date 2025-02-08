use macros::RmcSerialize;
use rand::random;
use crate::prudp::packet::{PRUDPHeader, PRUDPPacket, TypesFlags};
use crate::prudp::packet::flags::{NEED_ACK, RELIABLE};
use crate::prudp::packet::types::DATA;
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::message::RMCMessage;
use crate::rmc::structures::RmcSerialize;

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct Notification{
    pub pid_source: u32,
    pub notif_type: u32,
    pub param_1: u32,
    pub param_2: u32,
    pub str_param: String,
    pub param_3: u32,
}

impl ConnectionData{
    pub async fn send_notification(&mut self, socket: &SocketData, notif: Notification){
        println!("sending notification");

        let mut data = Vec::new();

        notif.serialize(&mut data).expect("unable to write");

        let message = RMCMessage{
            protocol_id: 14,
            method_id: 1,
            call_id: random(),
            rest_of_data: data
        };

        let prudp_packet = PRUDPPacket{
            header: PRUDPHeader{
                types_and_flags: TypesFlags::default().types(DATA).flags(NEED_ACK | RELIABLE),
                source_port: socket.get_virual_port(),
                destination_port: self.sock_addr.virtual_port,
                ..Default::default()
            },
            options: Vec::new(),
            payload: message.to_data(),
            packet_signature: [0;16]
        };



        self.finish_and_send_packet_to(socket, prudp_packet).await;
    }
}