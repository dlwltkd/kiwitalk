use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Request<'a> {
    #[serde(rename = "k")]
    pub token: &'a str,

    #[serde(rename = "t")]
    pub log_type: i32,

    #[serde(rename = "c")]
    pub chat_id: Option<i64>,

    #[serde(rename = "rt")]
    pub resource_type: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    #[serde(rename = "vh")]
    pub vhost: String,

    #[serde(rename = "vh6")]
    pub vhost6: Option<String>,

    #[serde(rename = "p")]
    pub port: i32,

    #[serde(default, rename = "rd")]
    pub redirect: bool,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_get_trailer_uses_conditional_context_fields() {
        let minimal = bson::to_document(&Request {
            token: "token",
            log_type: 2,
            chat_id: None,
            resource_type: None,
        })
        .unwrap();
        assert_eq!(
            minimal.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from(["k", "t"])
        );

        let contextual = bson::to_document(&Request {
            token: "token",
            log_type: 2,
            chat_id: Some(3),
            resource_type: Some("image"),
        })
        .unwrap();
        assert_eq!(contextual.get_i64("c"), Ok(3));
        assert_eq!(contextual.get_str("rt"), Ok("image"));
    }

    #[test]
    fn android_get_trailer_allows_missing_ipv6_host() {
        let response: Response = bson::from_document(bson::doc! {
            "p": 443_i32,
            "vh": "127.0.0.1",
        })
        .unwrap();

        assert_eq!(response.vhost6, None);
        assert!(!response.redirect);
    }
}
