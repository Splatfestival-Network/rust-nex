mod method_login_ex;
mod method_login;

use log::{error};
use crate::define_protocol;
use crate::nex::account::Account;
use crate::protocols::auth::method_login::login_raw_params;
use crate::protocols::auth::method_login_ex::{login_ex, login_ex_raw_params};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse};


define_protocol!{
    10(secure_server_account: &'static Account) => {
        0x01 => login_raw_params,
        0x02 => login_ex_raw_params
    }
}
