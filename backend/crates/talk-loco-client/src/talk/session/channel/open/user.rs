use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayUser {
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(rename = "nickName")]
    pub nickname: String,

    #[serde(rename = "profileImageUrl")]
    pub profile_image_url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(rename = "nickName")]
    pub nickname: String,

    #[serde(rename = "pi")]
    pub profile_image_url: String,

    #[serde(rename = "fpi")]
    pub full_profile_image_url: String,

    #[serde(rename = "opi")]
    pub original_profile_image_url: String,

    #[serde(rename = "type")]
    pub user_type: i32,

    #[serde(rename = "accountId")]
    pub account_id: i64,

    #[serde(rename = "countryIso")]
    pub country_iso: Option<String>,

    #[serde(rename = "ut")]
    pub service_user_type: Option<i32>,

    pub suspended: bool,

    pub suspicion: String,

    #[serde(rename = "accessPermit")]
    pub access_permit: Option<String>,

    #[serde(rename = "mt")]
    pub open_member_type: i32,

    #[serde(rename = "ptp")]
    pub profile_type: i32,

    #[serde(rename = "pli")]
    pub profile_link_id: i64,

    /// Retained for compatibility with responses that include the older field.
    #[serde(rename = "opt")]
    pub open_token: i32,

    #[serde(rename = "pfId")]
    pub pf_id: Option<i64>,
}

#[derive(Deserialize)]
struct UserWire {
    #[serde(rename = "userId")]
    user_id: i64,

    #[serde(rename = "nickName")]
    nickname: String,

    #[serde(rename = "type")]
    user_type: i32,

    #[serde(default, rename = "profileImageUrl")]
    profile_image_url: Option<String>,

    #[serde(default, rename = "pi")]
    short_profile_image_url: Option<String>,

    #[serde(default, rename = "fullProfileImageUrl")]
    full_profile_image_url: Option<String>,

    #[serde(default, rename = "fpi")]
    short_full_profile_image_url: Option<String>,

    #[serde(default, rename = "originalProfileImageUrl")]
    original_profile_image_url: Option<String>,

    #[serde(default, rename = "opi")]
    short_original_profile_image_url: Option<String>,

    #[serde(default, rename = "accountId")]
    account_id: i64,

    #[serde(default, rename = "countryIso")]
    country_iso: Option<String>,

    #[serde(default, rename = "ut")]
    service_user_type: Option<i32>,

    #[serde(default)]
    suspended: bool,

    #[serde(default = "default_suspicion")]
    suspicion: String,

    #[serde(default, rename = "accessPermit")]
    access_permit: Option<String>,

    #[serde(default, rename = "mt")]
    open_member_type: i32,

    #[serde(default = "default_profile_type", rename = "ptp")]
    profile_type: i32,

    #[serde(default, rename = "pli")]
    profile_link_id: i64,

    #[serde(default = "default_open_token", rename = "opt")]
    open_token: i32,

    #[serde(default, rename = "pfId")]
    pf_id: Option<i64>,
}

fn default_suspicion() -> String {
    "UNKNOWN".to_owned()
}

fn default_profile_type() -> i32 {
    -1
}

fn default_open_token() -> i32 {
    -1
}

impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = UserWire::deserialize(deserializer)?;

        Ok(Self {
            user_id: wire.user_id,
            nickname: wire.nickname,
            profile_image_url: wire
                .profile_image_url
                .or(wire.short_profile_image_url)
                .unwrap_or_default(),
            full_profile_image_url: wire
                .full_profile_image_url
                .or(wire.short_full_profile_image_url)
                .unwrap_or_default(),
            original_profile_image_url: wire
                .original_profile_image_url
                .or(wire.short_original_profile_image_url)
                .unwrap_or_default(),
            user_type: wire.user_type,
            account_id: wire.account_id,
            country_iso: wire.country_iso,
            service_user_type: wire.service_user_type,
            suspended: wire.suspended,
            suspicion: wire.suspicion,
            access_permit: wire.access_permit,
            open_member_type: wire.open_member_type,
            profile_type: wire.profile_type,
            profile_link_id: wire.profile_link_id,
            open_token: wire.open_token,
            pf_id: wire.pf_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_sparse_current_open_member() {
        let user: User = bson::from_document(bson::doc! {
            "userId": 1_i64,
            "nickName": "member",
            "type": 2_i32,
        })
        .unwrap();

        assert!(user.profile_image_url.is_empty());
        assert_eq!(user.account_id, 0);
        assert_eq!(user.open_member_type, 0);
        assert_eq!(user.profile_type, -1);
        assert_eq!(user.profile_link_id, 0);
        assert_eq!(user.open_token, -1);
        assert_eq!(user.suspicion, "UNKNOWN");
    }

    #[test]
    fn prefers_long_profile_names_and_accepts_short_fallbacks() {
        let long: User = bson::from_document(bson::doc! {
            "userId": 1_i64,
            "nickName": "member",
            "type": 2_i32,
            "profileImageUrl": "long",
            "pi": "short",
        })
        .unwrap();
        assert_eq!(long.profile_image_url, "long");

        let short: User = bson::from_document(bson::doc! {
            "userId": 1_i64,
            "nickName": "member",
            "type": 2_i32,
            "pi": "short",
        })
        .unwrap();
        assert_eq!(short.profile_image_url, "short");
    }
}
