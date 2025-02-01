

pub struct Account{
    pid: u32,
    username: Box<str>,
    kerbros_password: Box<str>,

}

impl Account{
    pub fn new(pid: u32, username: &str, passwd: &str) -> Self{
        Self{
            kerbros_password: passwd.into(),
            username: username.into(),
            pid
        }
    }
}