use std::io::Cursor;
use log::{error, info};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse, RMCResponseResult};
use crate::rmc::structures::{string, any, RmcSerialize};
use crate::rmc::structures::any::Any;

pub fn login_ex(name: &str) -> RMCResponseResult{
    // todo: figure out how the AuthenticationInfo struct works, parse it and validate login info

    //return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    unreachable!()
}

pub fn login_ex_raw_params(rmcmessage: &RMCMessage) -> RMCResponseResult{
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

    //login_ex(&str)
    rmcmessage.error_result_with_code(ErrorCode::Authentication_UnderMaintenance)
}