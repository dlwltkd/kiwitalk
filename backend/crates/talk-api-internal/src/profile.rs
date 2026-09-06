use reqwest::Method;
use serde::Deserialize;

use crate::{client::ApiClient, read_structured_response, ApiResult};

#[derive(Debug, Clone)]
pub struct MeProfile {
    pub nickname: String,
    pub user_id: u64,
    pub background_image_url: String,
    pub original_background_image_url: String,
    pub status_message: String,
    pub profile_image_url: String,
    pub full_profile_image_url: String,
    pub original_profile_image_url: String,
}

#[derive(Debug, Clone)]
pub struct Me {
    pub profile: MeProfile,
}

impl Me {
    pub async fn request(
        client: ApiClient<'_>,
        profile_id: u64,
        last_seen_at: u64,
    ) -> ApiResult<Self> {
        let response: MeWire = read_structured_response(
            client
                .request_pilsner(Method::GET, "profile25/me")?
                .query(&[("profileId", profile_id), ("lastSeenAt", last_seen_at)]),
        )
        .await?;

        Ok(response.into())
    }
}

#[derive(Debug, Clone)]
pub struct FriendProfile {
    pub user_id: u64,
    pub background_image_url: String,
    pub original_background_image_url: String,
    pub status_message: String,
    pub profile_image_url: String,
    pub full_profile_image_url: String,
    pub original_profile_image_url: String,
}

#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub profile: FriendProfile,
}

impl FriendInfo {
    pub async fn request(client: ApiClient<'_>, id: u64) -> ApiResult<Self> {
        let response: OtherWire = read_structured_response(
            client
                .request_pilsner(Method::GET, "profile25/other")?
                .query(&[("userId", id)]),
        )
        .await?;

        Ok(response.into())
    }
}

#[derive(Debug, Deserialize)]
struct ProfileImageWire {
    #[serde(default, rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
    #[serde(default, rename = "mediumUrl")]
    medium_url: Option<String>,
    #[serde(default, rename = "originalUrl")]
    original_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BackgroundImageWire {
    #[serde(default, rename = "mediumUrl")]
    medium_url: Option<String>,
    #[serde(default, rename = "originalUrl")]
    original_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusMessageWire {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MeWire {
    #[serde(rename = "userId")]
    user_id: u64,
    nickname: String,
    #[serde(rename = "profileImage")]
    profile_image: ProfileImageWire,
    #[serde(rename = "backgroundImage")]
    background_image: BackgroundImageWire,
    #[serde(rename = "statusMessage")]
    status_message: StatusMessageWire,
}

#[derive(Debug, Deserialize)]
struct OtherWire {
    #[serde(rename = "userId")]
    user_id: u64,
    #[serde(rename = "profileImage")]
    profile_image: ProfileImageWire,
    #[serde(rename = "backgroundImage")]
    background_image: BackgroundImageWire,
    #[serde(rename = "statusMessage")]
    status_message: StatusMessageWire,
}

fn profile_urls(image: ProfileImageWire) -> (String, String, String) {
    let original = image
        .original_url
        .or_else(|| image.medium_url.clone())
        .or_else(|| image.thumbnail_url.clone())
        .unwrap_or_default();
    let medium = image.medium_url.unwrap_or_else(|| original.clone());
    let thumbnail = image.thumbnail_url.unwrap_or_else(|| medium.clone());

    (thumbnail, medium, original)
}

fn background_urls(image: BackgroundImageWire) -> (String, String) {
    let original = image
        .original_url
        .or_else(|| image.medium_url.clone())
        .unwrap_or_default();
    let medium = image.medium_url.unwrap_or_else(|| original.clone());

    (medium, original)
}

impl From<MeWire> for Me {
    fn from(value: MeWire) -> Self {
        let (profile_image_url, full_profile_image_url, original_profile_image_url) =
            profile_urls(value.profile_image);
        let (background_image_url, original_background_image_url) =
            background_urls(value.background_image);

        Self {
            profile: MeProfile {
                nickname: value.nickname,
                user_id: value.user_id,
                background_image_url,
                original_background_image_url,
                status_message: value.status_message.message.unwrap_or_default(),
                profile_image_url,
                full_profile_image_url,
                original_profile_image_url,
            },
        }
    }
}

impl From<OtherWire> for FriendInfo {
    fn from(value: OtherWire) -> Self {
        let (profile_image_url, full_profile_image_url, original_profile_image_url) =
            profile_urls(value.profile_image);
        let (background_image_url, original_background_image_url) =
            background_urls(value.background_image);

        Self {
            profile: FriendProfile {
                user_id: value.user_id,
                background_image_url,
                original_background_image_url,
                status_message: value.status_message.message.unwrap_or_default(),
                profile_image_url,
                full_profile_image_url,
                original_profile_image_url,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FriendInfo, Me, MeWire, OtherWire};

    #[test]
    fn profile25_me_response_maps_nested_media() {
        let wire: MeWire = serde_json::from_str(
            r#"{
                "status": 0,
                "userId": 42,
                "profileId": 7,
                "suspended": false,
                "nickname": "Kiwi",
                "profileImage": {
                    "thumbnailUrl": "thumbnail",
                    "mediumUrl": "medium",
                    "originalUrl": "original",
                    "newBadge": 0
                },
                "backgroundImage": {
                    "mediumUrl": "background-medium",
                    "originalUrl": "background-original",
                    "newBadge": 0
                },
                "statusMessage": { "message": "hello", "newBadge": 0 }
            }"#,
        )
        .unwrap();
        let profile = Me::from(wire).profile;

        assert_eq!(profile.user_id, 42);
        assert_eq!(profile.profile_image_url, "thumbnail");
        assert_eq!(profile.full_profile_image_url, "medium");
        assert_eq!(profile.original_profile_image_url, "original");
        assert_eq!(profile.background_image_url, "background-medium");
        assert_eq!(profile.status_message, "hello");
    }

    #[test]
    fn profile25_other_response_accepts_null_media_and_status() {
        let wire: OtherWire = serde_json::from_str(
            r#"{
                "status": 0,
                "userId": 84,
                "suspended": false,
                "nickname": "Friend",
                "profileImage": {
                    "thumbnailUrl": null,
                    "mediumUrl": null,
                    "originalUrl": null,
                    "newBadge": 0
                },
                "backgroundImage": {
                    "mediumUrl": null,
                    "originalUrl": null,
                    "newBadge": 0
                },
                "statusMessage": { "message": null, "newBadge": 0 }
            }"#,
        )
        .unwrap();
        let profile = FriendInfo::from(wire).profile;

        assert_eq!(profile.user_id, 84);
        assert!(profile.profile_image_url.is_empty());
        assert!(profile.background_image_url.is_empty());
        assert!(profile.status_message.is_empty());
    }
}
