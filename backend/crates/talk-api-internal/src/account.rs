use reqwest::Method;
use serde::Deserialize;

use crate::{client::ApiClient, read_structured_response, ApiResult};

#[derive(Debug, Clone)]
pub struct MoreSettings {
    pub since: u64,

    pub account_id: u64,
    pub account_display_id: String,
    pub hashed_account_id: String,

    pub pstn_number: String,
    pub formatted_pstn_number: String,
    pub nsn_number: String,
    pub formatted_nsn_number: String,

    pub email_address: String,
    pub email_verified: bool,

    pub uuid: Option<String>,
    pub uuid_searchable: bool,
    pub nickname: String,

    pub profile_image_url: String,
    pub full_profile_image_url: String,
    pub original_profile_image_url: String,

    pub status_message: String,
}

#[derive(Debug, Default, Deserialize)]
struct AccountProfile {
    #[serde(default, rename = "statusMessage")]
    status_message: Option<String>,
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default, rename = "profileImageUrl")]
    profile_image_url: Option<String>,
    #[serde(default, rename = "fullProfileImageUrl")]
    full_profile_image_url: Option<String>,
    #[serde(default, rename = "originalProfileImageUrl")]
    original_profile_image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MoreSettingsWire {
    #[serde(default)]
    since: u64,

    #[serde(default, rename = "accountId")]
    account_id: Option<u64>,
    #[serde(default, rename = "accountDisplayId")]
    account_display_id: Option<String>,
    #[serde(default, rename = "hashedAccountId")]
    hashed_account_id: Option<String>,

    #[serde(default, rename = "phoneNumber", alias = "pstnNumber")]
    phone_number: Option<String>,
    #[serde(default, rename = "formattedPstnNumber")]
    formatted_pstn_number: Option<String>,
    #[serde(default, rename = "nsnNumber")]
    nsn_number: Option<String>,
    #[serde(default, rename = "formattedNsnNumber")]
    formatted_nsn_number: Option<String>,

    #[serde(default, rename = "emailAddress")]
    email_address: Option<String>,
    #[serde(default, rename = "emailVerified")]
    email_verified: bool,

    #[serde(default)]
    uuid: Option<String>,
    #[serde(default, rename = "uuidSearchable")]
    uuid_searchable: bool,

    #[serde(default)]
    profile: Option<AccountProfile>,

    // Older responses exposed these fields at the top level. Current Android
    // responses place the same values under `profile`.
    #[serde(default, rename = "nickName")]
    nickname: Option<String>,
    #[serde(default, rename = "profileImageUrl")]
    profile_image_url: Option<String>,
    #[serde(default, rename = "fullProfileImageUrl")]
    full_profile_image_url: Option<String>,
    #[serde(default, rename = "originalProfileImageUrl")]
    original_profile_image_url: Option<String>,
    #[serde(default, rename = "statusMessage")]
    status_message: Option<String>,
}

impl From<MoreSettingsWire> for MoreSettings {
    fn from(value: MoreSettingsWire) -> Self {
        let profile = value.profile.unwrap_or_default();

        Self {
            since: value.since,
            account_id: value.account_id.unwrap_or_default(),
            account_display_id: value.account_display_id.unwrap_or_default(),
            hashed_account_id: value.hashed_account_id.unwrap_or_default(),
            pstn_number: value.phone_number.unwrap_or_default(),
            formatted_pstn_number: value.formatted_pstn_number.unwrap_or_default(),
            nsn_number: value.nsn_number.unwrap_or_default(),
            formatted_nsn_number: value.formatted_nsn_number.unwrap_or_default(),
            email_address: value.email_address.unwrap_or_default(),
            email_verified: value.email_verified,
            uuid: value.uuid,
            uuid_searchable: value.uuid_searchable,
            nickname: profile.nickname.or(value.nickname).unwrap_or_default(),
            profile_image_url: profile
                .profile_image_url
                .or(value.profile_image_url)
                .unwrap_or_default(),
            full_profile_image_url: profile
                .full_profile_image_url
                .or(value.full_profile_image_url)
                .unwrap_or_default(),
            original_profile_image_url: profile
                .original_profile_image_url
                .or(value.original_profile_image_url)
                .unwrap_or_default(),
            status_message: profile
                .status_message
                .or(value.status_message)
                .unwrap_or_default(),
        }
    }
}

impl MoreSettings {
    pub async fn request(client: ApiClient<'_>) -> ApiResult<Self> {
        let response: MoreSettingsWire =
            read_structured_response(client.request(Method::GET, "account/more_settings.json")?)
                .await?;

        Ok(response.into())
    }
}

#[cfg(test)]
mod tests {
    use super::{MoreSettings, MoreSettingsWire};

    #[test]
    fn current_nested_profile_is_flattened_for_callers() {
        let wire: MoreSettingsWire = serde_json::from_str(
            r#"{
                "status": 0,
                "since": 17,
                "accountId": 42,
                "accountDisplayId": "display",
                "hashedAccountId": "hash",
                "phoneNumber": "+821012345678",
                "formattedPstnNumber": "010-1234-5678",
                "nsnNumber": "1012345678",
                "emailAddress": "user@example.com",
                "emailVerified": true,
                "uuid": "kakao-id",
                "uuidSearchable": true,
                "profile": {
                    "statusMessage": "hello",
                    "nickname": "Kiwi",
                    "profileId": "99",
                    "profileImageUrl": "small",
                    "fullProfileImageUrl": "medium",
                    "originalProfileImageUrl": "original"
                }
            }"#,
        )
        .unwrap();
        let settings = MoreSettings::from(wire);

        assert_eq!(settings.nickname, "Kiwi");
        assert_eq!(settings.status_message, "hello");
        assert_eq!(settings.profile_image_url, "small");
        assert_eq!(settings.full_profile_image_url, "medium");
        assert_eq!(settings.original_profile_image_url, "original");
        assert_eq!(settings.pstn_number, "+821012345678");
        assert!(settings.uuid_searchable);
    }
}
