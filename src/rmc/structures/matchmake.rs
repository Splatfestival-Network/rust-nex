use macros::RmcSerialize;
use crate::kerberos::KerberosDateTime;
use crate::rmc::structures::RmcSerialize;
use crate::rmc::structures::variant::Variant;

// rmc structure
#[derive(RmcSerialize)]
#[rmc_struct(0)]
struct Gathering{
    self_gid: u32,
    owner_pid: u32,
    host_pid: u32,
    minimum_participants: u16,
    maximum_participants: u16,
    participant_policy: u32,
    policy_argument: u32,
    flags: u32,
    state: u32,
    description: String
}

// rmc structure
#[derive(RmcSerialize)]
#[rmc_struct(0)]
struct MatchmakeParam{
    params: Vec<(String, Variant)>
}


// rmc structure
#[derive(RmcSerialize)]
#[rmc_struct(3)]
struct MatchmakeSession{
    //inherits from
    #[extends]
    gathering: Gathering,

    gamemode: u32,
    attributes: Vec<u32>,
    open_participation: bool,
    matchmake_system_type: u32,
    application_buffer: Vec<u8>,
    participation_count: u32,
    progress_score: u8,
    session_key: Vec<u8>,
    option0: u32,
    matchmake_param: MatchmakeParam,
    datetime: KerberosDateTime,
    user_password: String,
    refer_gid: u32,
    user_password_enabled: bool,
    system_password_enabled: bool
}

