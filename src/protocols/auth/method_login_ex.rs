use std::io::Cursor;
use log::{error, info};
use crate::grpc::account;
use crate::nex::account::Account;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse, RMCResponseResult};
use crate::rmc::structures::{string, any, RmcSerialize};
use crate::rmc::structures::any::Any;

pub async fn login_ex(rmcmessage: &RMCMessage, secure_server_account: &Account , pid: u32) -> RMCResponseResult{
    // todo: figure out how the AuthenticationInfo struct works, parse it and validate login info

    let Ok(mut client) = account::Client::new().await else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    let Ok(passwd) = client.get_nex_password(pid).await else{
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    

    return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
}

pub async fn login_ex_raw_params(rmcmessage: &RMCMessage, (secure_server_account): (&Account)) -> RMCResponseResult{
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

    login_ex(rmcmessage, secure_server_account, pid).await
}