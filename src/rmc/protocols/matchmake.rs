use macros::{method_id, rmc_proto};
use crate::prudp::station_url::StationUrl;
use crate::rmc::response::ErrorCode;

#[rmc_proto(21)]
pub trait Matchmake{
    #[method_id(2)]
    async fn unregister_gathering(&self, gid: u32) -> Result<bool, ErrorCode>;
    #[method_id(41)]
    async fn get_session_urls(&self, gid: u32) -> Result<Vec<StationUrl>, ErrorCode>;
}