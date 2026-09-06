use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Credential<'a> {
    pub device_uuid: &'a str,
    pub access_token: &'a str,
}

impl fmt::Debug for Credential<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Credential(REDACTED)")
    }
}
