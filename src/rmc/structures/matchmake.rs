use macros::RmcSerialize;
use crate::kerberos::KerberosDateTime;
use crate::rmc::structures::RmcSerialize;
use crate::rmc::structures::variant::Variant;

// rmc structure
#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct Gathering {
    pub self_gid: u32,
    pub owner_pid: u32,
    pub host_pid: u32,
    pub minimum_participants: u16,
    pub maximum_participants: u16,
    pub participant_policy: u32,
    pub policy_argument: u32,
    pub flags: u32,
    pub state: u32,
    pub description: String,
}

// rmc structure
#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct MatchmakeParam {
    pub params: Vec<(String, Variant)>,
}


// rmc structure
#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(3)]
pub struct MatchmakeSession {
    //inherits from
    #[extends]
    pub gathering: Gathering,

    pub gamemode: u32,
    pub attributes: Vec<u32>,
    pub open_participation: bool,
    pub matchmake_system_type: u32,
    pub application_buffer: Vec<u8>,
    pub participation_count: u32,
    pub progress_score: u8,
    pub session_key: Vec<u8>,
    pub option0: u32,
    pub matchmake_param: MatchmakeParam,
    pub datetime: KerberosDateTime,
    pub user_password: String,
    pub refer_gid: u32,
    pub user_password_enabled: bool,
    pub system_password_enabled: bool,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(3)]
pub struct MatchmakeSessionSearchCriteria {
    pub attribs: Vec<String>,
    pub game_mode: String,
    pub minimum_participants: String,
    pub maximum_participants: String,
    pub matchmake_system_type: String,
    pub vacant_only: bool,
    pub exclude_locked: bool,
    pub exclude_non_host_pid: bool,
    pub selection_method: u32,
    pub vacant_participants: u16,
    pub matchmake_param: MatchmakeParam,
    pub exclude_user_password_set: bool,
    pub exclude_system_password_set: bool,
    pub refer_gid: u32,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct AutoMatchmakeParam {
    pub matchmake_session: MatchmakeSession,
    pub additional_participants: Vec<u32>,
    pub gid_for_participation_check: u32,
    pub auto_matchmake_option: u32,
    pub join_message: String,
    pub participation_count: u16,
    pub search_criteria: Vec<MatchmakeSessionSearchCriteria>,
    pub target_gids: Vec<u32>,
}