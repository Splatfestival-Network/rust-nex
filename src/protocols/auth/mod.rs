mod method_login_ex;

use log::{error, info};
use crate::protocols::auth::method_login_ex::{login_ex, login_ex_raw_params};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse, RMCResponseResult};

pub fn try_process_via_protocol(rmcmessage: &RMCMessage) -> Option<RMCResponse>{
    if rmcmessage.protocol_id != 10{
        return None;
    }

    let response_result = match rmcmessage.method_id{
        0x02 => login_ex_raw_params(rmcmessage),
        _ => {
            error!("invalid method id sent to ticket-granting protocol: {:?}", rmcmessage.method_id);
            rmcmessage.error_result_with_code(ErrorCode::Core_Exception)
        }
    };

    Some(RMCResponse{
        protocol_id: 10,
        response_result
    })
}