mod method_unregister_gathering;

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::define_protocol;
use crate::protocols::matchmake::method_unregister_gathering::unregister_gathering_raw_params;
use crate::protocols::matchmake_common::MatchmakeData;

define_protocol!{
    21(matchmake_data: Arc<RwLock<MatchmakeData>>) => {
        2 => unregister_gathering_raw_params
    }
}