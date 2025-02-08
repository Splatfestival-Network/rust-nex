mod method_get_playing_session;
mod method_auto_matchmake_with_param_postpone;
mod method_create_matchmake_session_with_param;

use std::sync::Arc;
use tokio::sync::{RwLock};
use crate::define_protocol;
use crate::protocols::matchmake_common::MatchmakeData;
use method_get_playing_session::get_playing_session_raw_params;
use method_auto_matchmake_with_param_postpone::auto_matchmake_with_param_postpone_raw_params;
use crate::protocols::matchmake_extension::method_create_matchmake_session_with_param::create_matchmake_session_with_param_raw_params;

define_protocol!{
    109(matchmake_data: Arc<RwLock<MatchmakeData>>) => {
        16 => get_playing_session_raw_params,
        38 => create_matchmake_session_with_param_raw_params,
        40 => auto_matchmake_with_param_postpone_raw_params
    }
}