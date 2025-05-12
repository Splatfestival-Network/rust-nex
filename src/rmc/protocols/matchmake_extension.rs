use macros::{method_id, rmc_proto};
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::matchmake::{AutoMatchmakeParam, CreateMatchmakeSessionParam, JoinMatchmakeSessionParam, MatchmakeSession};

#[rmc_proto(109)]
pub trait MatchmakeExtension{
    #[method_id(16)]
    async fn get_playing_session(&self, pids: Vec<u32>) -> Result<Vec<()>, ErrorCode>;

    #[method_id(34)]
    async fn update_progress_score(&self, gid: u32, progress: u8) -> Result<(), ErrorCode>;
    #[method_id(38)]
    async fn create_matchmake_session_with_param(&self, session: CreateMatchmakeSessionParam) -> Result<MatchmakeSession, ErrorCode>;

    #[method_id(39)]
    async fn join_matchmake_session_with_param(&self, session: JoinMatchmakeSessionParam) -> Result<MatchmakeSession, ErrorCode>;

    #[method_id(40)]
    async fn auto_matchmake_with_param_postpone(&self, session: AutoMatchmakeParam) -> Result<MatchmakeSession, ErrorCode>;
}