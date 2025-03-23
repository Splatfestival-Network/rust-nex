use crate::rmc::response::ErrorCode;
use crate::rmc::structures::any::Any;
use crate::rmc::structures::connection_data::ConnectionData;
use crate::rmc::structures::qresult::QResult;
use macros::{method_id, rmc_proto};


/// This is the representation for `Ticket Granting`(for details see the 
/// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
#[rmc_proto(10)]
pub trait Auth {
    #[method_id(1)]
    async fn login(&self, name: String) -> Result<(), ErrorCode>;

    #[method_id(2)]
    async fn login_ex(
        &self,
        name: String,
        extra_data: Any,
    ) -> Result<(QResult, u32, Vec<u8>, ConnectionData, String), ErrorCode>;

    #[method_id(3)]
    async fn request_ticket(
        &self,
        source_pid: u32,
        destination_pid: u32,
    ) -> Result<(QResult, Vec<u8>), ErrorCode>;

    #[method_id(4)]
    async fn get_pid(&self, username: String) -> Result<u32, ErrorCode>;

    #[method_id(5)]
    async fn get_name(&self, pid: u32) -> Result<String, ErrorCode>;
}
