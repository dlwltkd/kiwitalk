use headless_talk::init::config::NetworkType;

pub const TALK_NET_TYPE: NetworkType = NetworkType::Wired;

pub const BOOKING_HOST: &str = "booking-loco.kakao.com";
pub const BOOKING_SERVER: (&str, u16) = (BOOKING_HOST, 443);
