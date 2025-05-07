mod method_login_ex;
mod method_login;
mod ticket_generation;
mod method_request_ticket;

use crate::define_protocol;
use crate::grpc::account;
use crate::nex::account::Account;
use crate::protocols::auth::method_login::login_raw_params;
use crate::protocols::auth::method_login_ex::login_ex_raw_params;
use crate::protocols::auth::method_request_ticket::request_ticket_raw_params;

#[derive(Copy, Clone)]
pub struct AuthProtocolConfig {
    pub secure_server_account: &'static Account,
    pub build_name: &'static str,
    pub station_url: &'static str
}

define_protocol!{
    10(proto_data: AuthProtocolConfig) => {
        0x01 => login_raw_params,
        0x02 => login_ex_raw_params,
        0x03 => request_ticket_raw_params
    }
}

async fn get_login_data_by_pid(pid: u32) -> Option<(u32, [u8; 16])> {
    let Ok(mut client) = account::Client::new().await else {
        return None
    };

    let Ok(passwd) = client.get_nex_password(pid).await else{
        return None
    };

    Some((pid, passwd))
}