mod method_login_ex;

use log::{error, info};
use crate::define_protocol;
use crate::nex::account::Account;
use crate::protocols::auth::method_login_ex::{login_ex, login_ex_raw_params};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse, RMCResponseResult};


define_protocol!{
    10<'a>(account: &'a Account) => {
        0x02 => login_ex_raw_params
    }
}