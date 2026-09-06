use futures_lite::{AsyncRead, AsyncWrite};
use futures_loco_protocol::{loco_protocol::command::Method, LocoClient};
use serde::{Deserialize, Serialize};

use crate::RequestResult;

use super::request_simple;

fn is_false(value: &bool) -> bool {
    !value
}

fn is_blank(value: &&str) -> bool {
    value.trim().is_empty()
}

#[derive(Debug)]
pub struct CheckinClient<T>(LocoClient<T>);

impl<T> CheckinClient<T> {
    pub const fn new(client: LocoClient<T>) -> Self {
        Self(client)
    }

    pub fn into_inner(self) -> LocoClient<T> {
        self.0
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> CheckinClient<T> {
    pub async fn checkin(&mut self, req: &CheckinReq<'_>) -> RequestResult<CheckinRes> {
        request_simple(&mut self.0, Method::new("CHECKIN").unwrap(), req).await
    }

    pub async fn buy_cs(&mut self, req: &BuyCSReq<'_>) -> RequestResult<BuyCSRes> {
        request_simple(&mut self.0, Method::new("BUYCS").unwrap(), req).await
    }
}

/// Request loco server host data
#[derive(Debug, Clone, Serialize)]
pub struct CheckinReq<'a> {
    /// Client user id. Login to acquire.
    #[serde(rename = "userId")]
    pub user_id: i64,

    /// Current OS (win32, android, mac, etc.)
    pub os: &'a str,

    /// Network type (0 for Wi-Fi, 3 otherwise on Android).
    #[serde(rename = "ntype")]
    pub net_type: i16,

    /// Official app version
    #[serde(rename = "appVer")]
    pub app_version: &'a str,

    /// Network MCCMNC ("999" on desktop clients).
    #[serde(rename = "MCCMNC", skip_serializing_if = "is_blank")]
    pub mccmnc: &'a str,

    #[serde(rename = "lang")]
    pub language: &'a str,

    /// Subdevice(PC, Tablet) or not
    #[serde(rename = "useSub", skip_serializing_if = "is_false")]
    pub use_sub: bool,
}

/// Answer loco server information
#[derive(Debug, Clone, Deserialize)]
pub struct CheckinRes {
    /// Loco server ip
    #[serde(default)]
    pub host: String,

    /// Loco server ip(v6)
    #[serde(default)]
    pub host6: String,

    /// Loco server port
    #[serde(default)]
    pub port: i32,

    /// Info cache expire time(?)
    #[serde(rename = "cacheExpire")]
    #[serde(default)]
    pub cache_expire: i32,

    /// Call server ip
    #[serde(rename = "cshost")]
    #[serde(default)]
    pub cs_host: String,

    /// Call server ip(v6)
    #[serde(rename = "cshost6")]
    #[serde(default)]
    pub cs_host6: String,

    /// Call server port
    #[serde(rename = "csport")]
    #[serde(default)]
    pub cs_port: i32,

    /// Unknown server ip
    #[serde(rename = "vsshost")]
    #[serde(default)]
    pub vss_host: String,

    /// Unknown server ip(v6)
    #[serde(rename = "vsshost6")]
    #[serde(default)]
    pub vss_host6: String,

    /// Unknown server port
    #[serde(rename = "vssport")]
    #[serde(default)]
    pub vss_port: i32,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_checkin_serializes_the_26_7_2_field_set() {
        let request = CheckinReq {
            user_id: 42,
            os: "android",
            net_type: 0,
            app_version: "26.7.2",
            mccmnc: "999",
            language: "ko",
            use_sub: true,
        };

        let document = bson::to_document(&request).unwrap();
        let keys = document.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from(["MCCMNC", "appVer", "lang", "ntype", "os", "useSub", "userId"])
        );
    }

    #[test]
    fn optional_checkin_fields_are_omitted_when_inactive() {
        let request = CheckinReq {
            user_id: 42,
            os: "android",
            net_type: 0,
            app_version: "26.7.2",
            mccmnc: " ",
            language: "ko",
            use_sub: false,
        };

        let document = bson::to_document(&request).unwrap();
        assert!(!document.contains_key("MCCMNC"));
        assert!(!document.contains_key("useSub"));
    }

    #[test]
    fn current_checkin_response_accepts_absent_legacy_call_hosts() {
        let response: CheckinRes = bson::from_document(bson::doc! {
            "host": "127.0.0.1",
            "port": 443_i32,
        })
        .unwrap();

        assert_eq!(response.host, "127.0.0.1");
        assert_eq!(response.port, 443);
        assert!(response.cs_host.is_empty());
        assert_eq!(response.cs_port, 0);
    }
}

/// Request call server host data.
/// Checkin response already contains call server info
#[derive(Debug, Clone, Serialize)]
pub struct BuyCSReq<'a> {
    #[serde(rename = "userId")]
    pub user_id: i64,

    /// Current OS (win32, android, mac, etc.)
    pub os: &'a str,

    /// Network type (0 for wired)
    #[serde(rename = "ntype")]
    pub net_type: i16,

    /// Official app version
    #[serde(rename = "appVer")]
    pub app_version: &'a str,

    /// Network MCCMNC ("999" on pc)
    #[serde(rename = "MCCMNC")]
    pub mccmnc: &'a str,
}

/// Call server information
#[derive(Debug, Clone, Deserialize)]
pub struct BuyCSRes {
    /// Call server ip
    #[serde(default, rename = "cshost")]
    pub cs_host: String,

    /// Call server ip(v6)
    #[serde(default, rename = "cshost6")]
    pub cs_host6: String,

    /// Call server port
    #[serde(default, rename = "csport")]
    pub cs_port: i32,

    /// Unknown server ip
    #[serde(default, rename = "vsshost")]
    pub vss_host: String,

    /// Unknown server ip(v6)
    #[serde(default, rename = "vsshost6")]
    pub vss_host6: String,

    /// Unknown server port
    #[serde(default, rename = "vssport")]
    pub vss_port: i32,
}

#[cfg(test)]
mod buy_cs_tests {
    use super::*;

    #[test]
    fn android_buy_cs_serializes_the_current_field_order_and_types() {
        let request = BuyCSReq {
            user_id: 42,
            os: "android",
            net_type: 0,
            app_version: "26.7.2",
            mccmnc: "45008",
        };

        let document = bson::to_document(&request).unwrap();
        assert_eq!(
            document.keys().map(String::as_str).collect::<Vec<_>>(),
            ["userId", "os", "ntype", "appVer", "MCCMNC"]
        );
        assert_eq!(document.get_i64("userId").unwrap(), 42);
        assert_eq!(document.get_i32("ntype").unwrap(), 0);
        assert!(!document.contains_key("countryISO"));
    }

    #[test]
    fn android_buy_cs_accepts_the_current_vss_only_response() {
        let response: BuyCSRes = bson::from_document(bson::doc! {
            "vsshost": "127.0.0.1",
            "vsshost6": "::1",
            "vssport": 443_i32,
        })
        .unwrap();

        assert!(response.cs_host.is_empty());
        assert_eq!(response.cs_port, 0);
        assert_eq!(response.vss_host, "127.0.0.1");
        assert_eq!(response.vss_host6, "::1");
        assert_eq!(response.vss_port, 443);
    }
}
