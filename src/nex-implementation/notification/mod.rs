use macros::RmcSerialize;
use rand::random;
use crate::prudp::packet::{PRUDPHeader, PRUDPPacket, PacketOption, TypesFlags};
use crate::prudp::packet::flags::{NEED_ACK, RELIABLE};
use crate::prudp::packet::types::DATA;
use crate::rmc::message::RMCMessage;
use crate::rmc::structures::RmcSerialize;

#[derive(Debug, Eq, PartialEq, RmcSerialize)]
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
            call_id: 1,
            rest_of_data: data
        };

        println!("notif: {}", hex::encode(message.to_data()));


        let mut prudp_packet = PRUDPPacket{
            header: PRUDPHeader{
                types_and_flags: TypesFlags::default().types(DATA).flags(NEED_ACK | RELIABLE),
                source_port: socket.get_virual_port(),
                destination_port: self.sock_addr.virtual_port,
                ..Default::default()
            },
            options: vec![
                PacketOption::FragmentId(0),
            ],
            payload: message.to_data(),
            packet_signature: [0;16]
        };

        self.finish_and_send_packet_to(socket, prudp_packet).await;
    }
}

#[cfg(test)]
mod test{
    use std::io::Cursor;
    use rand::random;
    use crate::protocols::notification::Notification;
    use crate::prudp::packet::{PRUDPHeader, PRUDPPacket, PacketOption, TypesFlags};
    use crate::prudp::packet::flags::{NEED_ACK, RELIABLE};
    use crate::prudp::packet::types::DATA;
    use crate::rmc::message::RMCMessage;
    use crate::rmc::structures::RmcSerialize;

    #[test]
    fn test(){
        let data = hex::decode("ead001032900a1af62000000000000000000000000000000000000000000020100250000000e57238a6601000000001700000051b39957b90b00003661636851b3995701000001000000").unwrap();
        

        let packet = PRUDPPacket::new(&mut Cursor::new(data)).expect("invalid packet");

        println!("{:?}", packet);

        let rmc = RMCMessage::new(&mut Cursor::new(packet.payload)).expect("invalid rmc message");

        println!("{:?}", rmc);

        let notif = Notification::deserialize(&mut Cursor::new(rmc.rest_of_data)).expect("invalid notification");

        println!("{:?}", notif);
    }
    #[test]
    fn test2(){

        let data = hex::decode("250000000e57b6801001000000001700000051b39957b90b0000248a5a9851b3995701000001000000").unwrap();
        //let packet = PRUDPPacket::new(&mut Cursor::new(data)).expect("invalid packet");

        //println!("{:?}", packet);

        let rmc = RMCMessage::new(&mut Cursor::new(data)).expect("invalid rmc message");

        println!("{:?}", rmc);

        let notif = Notification::deserialize(&mut Cursor::new(rmc.rest_of_data)).expect("invalid notification");

        println!("{:?}", notif);
    }

    #[test]
    fn test_rmc_serialization(){
        let notif = Notification{
            pid_source: random(),
            notif_type: random(),
            param_1: random(),
            param_2: random(),
            str_param: "".to_string(),
            param_3: random(),
        };

        let mut notif_data = Vec::new();

        notif.serialize(&mut notif_data).unwrap();

        let message = RMCMessage{
            protocol_id: 14,
            method_id: 1,
            call_id: random(),
            rest_of_data: notif_data
        };

        let mut prudp_packet = PRUDPPacket{
            header: PRUDPHeader{
                ..Default::default()
            },
            options: vec![
                PacketOption::FragmentId(0),
            ],
            payload: message.to_data(),
            packet_signature: [0;16]
        };

        prudp_packet.set_sizes();



        let mut packet_data: Vec<u8> = Vec::new();

        prudp_packet.write_to(&mut packet_data).expect("what");

        let packet_deserialized = PRUDPPacket::new(&mut Cursor::new(packet_data)).unwrap();

        assert_eq!(prudp_packet, packet_deserialized);

        let message_deserialized = RMCMessage::new(&mut Cursor::new(packet_deserialized.payload)).unwrap();

        assert_eq!(message, message_deserialized);

        let notification_deserialized = Notification::deserialize(&mut Cursor::new(message_deserialized.rest_of_data)).unwrap();

        assert_eq!(notification_deserialized, notif);




    }
}