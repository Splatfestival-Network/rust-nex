mod method_register;
mod method_send_report;

use crate::define_protocol;
use crate::protocols::secure::method_register::register_raw_params;
use crate::protocols::secure::method_send_report::send_report_raw_params;

define_protocol!{
    11() => {
        0x01 => register_raw_params,
        0x08 => send_report_raw_params
    }
}