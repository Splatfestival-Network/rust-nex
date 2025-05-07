use macros::{method_id, rmc_proto};
use crate::prudp::station_url::StationUrl;
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::any::Any;
use crate::rmc::structures::connection_data::ConnectionData;
use crate::rmc::structures::qresult::QResult;

#[rmc_proto(11)]
pub trait Auth {
    #[method_id(1)]
    async fn register(&self, station_urls: Vec<String>) -> Result<(QResult, u32, String), ErrorCode>;
}
