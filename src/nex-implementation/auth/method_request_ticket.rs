use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::protocols::auth::{AuthProtocolConfig, get_login_data_by_pid};
use crate::protocols::auth::ticket_generation::generate_ticket;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::response::ErrorCode::Core_Unknown;
use crate::rmc::structures::qresult::QResult;
use crate::rmc::structures::RmcSerialize;

pub async fn request_ticket(rmcmessage: &RMCMessage, data: AuthProtocolConfig, source_pid: u32, destination_pid: u32) -> RMCResponseResult{
    let Some(source_login_data) = get_login_data_by_pid(source_pid).await else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    let desgination_login_data = if destination_pid == data.secure_server_account.pid{
        data.secure_server_account.get_login_data()
    } else {
        let Some(login) = get_login_data_by_pid(destination_pid).await else {
            return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
        };
        login
    };

    let result = QResult::success(Core_Unknown);

    let ticket = generate_ticket(source_login_data, desgination_login_data);

    let mut response: Vec<u8> = Vec::new();

    result.serialize(&mut response).expect("failed serializing result");
    ticket.serialize(&mut response).expect("failed serializing ticket");

    rmcmessage.success_with_data(response)
}

pub async fn request_ticket_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>,  _: &Arc<Mutex<ConnectionData>>, data: AuthProtocolConfig) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(source_pid) = reader.read_struct(IS_BIG_ENDIAN) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    let Ok(destination_pid) = reader.read_struct(IS_BIG_ENDIAN) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    request_ticket(rmcmessage, data, source_pid, destination_pid).await
}