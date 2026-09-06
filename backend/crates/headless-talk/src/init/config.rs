use talk_loco_client::talk::session::login::ResponseType;

#[derive(Debug, Clone, Copy)]
pub struct ClientEnv<'a> {
    pub os: &'a str,
    pub app_version: &'a str,
    pub net_type: NetworkType,
    pub mccmnc: &'a str,
    pub language: &'a str,
    pub protocol_version: &'a str,
    pub device_type: Option<i8>,
    pub revision: Option<i32>,
    pub include_pc_status: bool,
    pub background: Option<bool>,
    pub last_chat_id: Option<i64>,
    pub login_response_type: ResponseType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum NetworkType {
    Wired = 0,
}
