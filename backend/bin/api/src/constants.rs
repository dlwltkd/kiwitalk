use talk_api_internal::{agent::TalkApiAgent, auth::xvc::default::Win32XVCHasher};

pub const TALK_AGENT: TalkApiAgent = TalkApiAgent::Win32("10.0");
// June 2026 Win32 profile. The version and XVC seeds must change together.
pub const XVC_HASHER: Win32XVCHasher = Win32XVCHasher("JAYDEN", "JAYMOND");
pub const AUTO_LOGIN_KEY: (&str, &str) = ("PITT", "INORAN");

pub const TALK_VERSION: &str = "26.5.0";
