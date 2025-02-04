mod method_get_playing_session;
mod method_auto_matchmake_with_param_postpone;

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::define_protocol;
use crate::protocols::matchmake_common::MatchmakeData;
use method_get_playing_session::get_playing_session_raw_params;

define_protocol!{
    109(matchmake_data: Arc<Mutex<MatchmakeData>>) => {
        16 => get_playing_session_raw_params
    }
}