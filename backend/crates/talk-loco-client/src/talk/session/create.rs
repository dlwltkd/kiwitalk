use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Request<'a> {
    #[serde(rename = "memberIds")]
    pub user_ids: &'a [i64],

    #[serde(rename = "nickName", skip_serializing_if = "is_none_or_blank")]
    pub nickname: Option<&'a str>,

    #[serde(rename = "profileImageUrl", skip_serializing_if = "is_none_or_blank")]
    pub profile_image_url: Option<&'a str>,
}

fn is_none_or_blank(value: &Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseVariant {
    Done(Response),
    Exists(Response),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Response {
    #[serde(rename = "chatId")]
    pub channel_id: i64,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn android_create_omits_blank_optional_profile_fields() {
        let blank = bson::to_document(&Request {
            user_ids: &[1],
            nickname: Some("  "),
            profile_image_url: Some(""),
        })
        .unwrap();
        assert_eq!(blank, bson::doc! { "memberIds": [1_i64] });

        let populated = bson::to_document(&Request {
            user_ids: &[1],
            nickname: Some("room"),
            profile_image_url: Some("profile-path"),
        })
        .unwrap();
        let keys = populated
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            keys,
            BTreeSet::from(["memberIds", "nickName", "profileImageUrl"])
        );
    }
}
