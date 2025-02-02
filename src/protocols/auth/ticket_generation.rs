use crate::kerberos;
use crate::kerberos::{derive_key, Ticket};


pub fn generate_ticket(source_act_login_data: (u32, [u8;16]), dest_act_login_data: (u32, [u8;16])) -> Box<[u8]>{
    let source_key = derive_key(source_act_login_data.0, source_act_login_data.1);
    let dest_key = derive_key(dest_act_login_data.0, dest_act_login_data.1);

    let internal_data = kerberos::TicketInternalData::new(source_act_login_data.0);

    let encrypted_inner = internal_data.encrypt(dest_key);
    let encrypted_session_ticket = Ticket{
        pid: dest_act_login_data.0,
        session_key: internal_data.session_key,
    }.encrypt(source_key, &encrypted_inner);


    encrypted_session_ticket
}