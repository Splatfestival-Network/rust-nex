use std::io::Cursor;
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use crate::nex::account::Account;
use crate::protocols::auth::AuthProtocolConfig;
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::RmcSerialize;

pub async fn login(rmcmessage: &RMCMessage, _name: &str) -> RMCResponseResult{


    rmcmessage.error_result_with_code(ErrorCode::Core_NotImplemented)
}

pub async fn login_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, _: &Arc<Mutex<ConnectionData>>, data: AuthProtocolConfig) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(str) = String::deserialize(&mut reader) else {
        error!("error reading packet");
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };


    login(rmcmessage, &str).await
}