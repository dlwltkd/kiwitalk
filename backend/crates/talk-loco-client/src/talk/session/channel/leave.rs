use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<'a> {
    pub block: bool,
    pub from: &'a str,
    pub report: bool,
    pub link_id: Option<i64>,
    pub silence: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct Response {
    #[serde(default, rename = "lastTokenId")]
    pub last_token_id: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct WireRequest<'a> {
    #[serde(rename = "chatId")]
    chat_id: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    block: Option<bool>,

    #[serde(rename = "f")]
    from: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<bool>,

    #[serde(rename = "li", skip_serializing_if = "Option::is_none")]
    link_id: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    silence: Option<i64>,
}

impl<'a> WireRequest<'a> {
    pub(super) fn new(chat_id: i64, request: &'a Request<'a>) -> Self {
        Self {
            chat_id,
            block: request.block.then_some(true),
            from: request.from,
            report: request.report.then_some(true),
            link_id: request.report.then_some(request.link_id).flatten(),
            silence: request.silence.then_some(chat_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_leave_uses_current_conditional_fields() {
        let ordinary = bson::to_document(&WireRequest::new(
            10,
            &Request {
                block: false,
                from: "d|1000|",
                report: false,
                link_id: Some(20),
                silence: false,
            },
        ))
        .unwrap();
        assert_eq!(ordinary, bson::doc! { "chatId": 10_i64, "f": "d|1000|" });

        let reported = bson::to_document(&WireRequest::new(
            10,
            &Request {
                block: true,
                from: "od|1000|spam",
                report: true,
                link_id: Some(20),
                silence: true,
            },
        ))
        .unwrap();
        let keys = reported.keys().map(String::as_str).collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from(["block", "chatId", "f", "li", "report", "silence"])
        );
        assert!(reported.get_bool("block").unwrap());
        assert!(reported.get_bool("report").unwrap());
        assert_eq!(reported.get_i64("li").unwrap(), 20);
        assert_eq!(reported.get_i64("silence").unwrap(), 10);
    }

    #[test]
    fn android_leave_defaults_missing_last_token_id() {
        let response: Response = bson::from_document(bson::doc! {}).unwrap();
        assert_eq!(response.last_token_id, 0);
    }
}
