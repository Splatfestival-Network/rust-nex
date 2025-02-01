

struct Account{
    pid: u32,
    kerbros_password: Box<str>,
}

impl Account{
    fn new(pid: u32, passwd: &str) -> Self{
        Self{
            kerbros_password: passwd.into(),
            pid
        }
    }
}