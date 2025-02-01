use std::io::Cursor;
use log::error;
use crate::nex::account::Account;
use crate::protocols::auth::method_login_ex::login_ex;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::any::Any;
use crate::rmc::structures::RmcSerialize;

pub fn login(rmcmessage: &RMCMessage, name: &str) -> RMCResponseResult{
    rmcmessage.error_result_with_code(ErrorCode::Core_NotImplemented)
}

pub fn login_raw_params(rmcmessage: &RMCMessage, account: &Account) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(str) = String::deserialize(&mut reader) else {
        error!("error reading packet");
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };


    login(rmcmessage, &str)
}