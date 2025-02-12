mod method_report_nat_properties;

use crate::define_protocol;
use crate::protocols::nat_traversal::method_report_nat_properties::report_nat_properties_raw_params;

define_protocol!{
    3() => {
        5 => report_nat_properties_raw_params
    }
}