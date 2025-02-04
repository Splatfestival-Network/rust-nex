mod method_register;

use crate::define_protocol;
use crate::protocols::secure::method_register::register_raw_params;

define_protocol!{
    11() => {
        0x01 => register_raw_params
    }
}