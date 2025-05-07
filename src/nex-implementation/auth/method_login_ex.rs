use std::io::{Cursor, Write};
use std::sync::Arc;
use bytemuck::bytes_of;
use log::{error};
use tokio::sync::Mutex;
use crate::grpc::account;
use crate::kerberos::KerberosDateTime;
use crate::protocols::auth::AuthProtocolConfig;
use crate::protocols::auth::ticket_generation::generate_ticket;
use crate::rmc;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::{RmcSerialize};
use crate::rmc::structures::any::Any;
use crate::rmc::structures::qresult::QResult;

pub async fn login_ex(rmcmessage: &RMCMessage, proto_data: AuthProtocolConfig, pid: u32) -> RMCResponseResult{
    // todo: figure out how the AuthenticationInfo struct works, parse it and validate login info

    let Ok(mut client) = account::Client::new().await else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    let Ok(passwd) = client.get_nex_password(pid).await else{
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    let source_login_data = (pid, passwd);
    let destination_login_data = proto_data.secure_server_account.get_login_data();

    let ticket = generate_ticket(source_login_data, destination_login_data);

    let result = QResult::success(ErrorCode::Core_Unknown);

    let connection_data = rmc::structures::connection_data::ConnectionData{
        station_url: proto_data.station_url,
        special_station_url: "",
        date_time: KerberosDateTime::now(),
        special_protocols: Vec::new()
    };

    let mut response: Vec<u8> = Vec::new();

    result.serialize(&mut response).expect("failed serializing result");
    response.write_all(bytes_of(&source_login_data.0)).expect("failed writing pid");
    ticket.serialize(&mut response).expect("failed serializing ticket");
    connection_data.serialize(&mut response).expect("failed writing connection data");
    proto_data.build_name.serialize(&mut response).expect("failed writing build name");

    return rmcmessage.success_with_data(response);
}

pub async fn login_ex_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, _: &Arc<Mutex<ConnectionData>>, data: AuthProtocolConfig) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(str) =  String::deserialize(&mut reader) else {
        error!("error reading packet");
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    let Ok(any) =  Any::deserialize(&mut reader) else {
        error!("error reading packet");
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    match any.name.as_ref(){
        "AuthenticationInfo" => {

        }
        v => {
            error!("error reading packet: invalid structure type: {}", v);
            return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
        }
    }

    let Ok(pid) = str.parse() else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    login_ex(rmcmessage, data, pid).await
}