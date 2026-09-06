use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::{client::ApiClient, read_structured_response, ApiResult, RequestResult};

#[derive(Debug, Clone, Deserialize)]
pub struct DiffFriend {
    #[serde(rename = "userId")]
    pub user_id: u64,

    #[serde(rename = "nickName")]
    pub nickname: String,

    #[serde(default, rename = "friendNickName")]
    pub friend_nickname: Option<String>,

    #[serde(rename = "type")]
    pub user_type: i32,

    #[serde(rename = "userType")]
    pub user_category: i32,

    #[serde(rename = "statusMessage")]
    pub status_message: String,

    #[serde(rename = "profileImageUrl")]
    pub profile_image_url: String,

    #[serde(rename = "fullProfileImageUrl")]
    pub full_profile_image_url: String,

    #[serde(rename = "originalProfileImageUrl")]
    pub original_profile_image_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FriendsDiff {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub deleted_ids: Vec<u64>,
    #[serde(default)]
    pub added_friends: Vec<DiffFriend>,
}

impl FriendsDiff {
    pub async fn request(client: ApiClient<'_>, ids: &[u64]) -> ApiResult<Self> {
        read_structured_response(diff_request(client, ids)?).await
    }
}

#[derive(Serialize)]
struct DiffForm<'a> {
    friend_ids: &'a str,
}

fn diff_request(client: ApiClient<'_>, ids: &[u64]) -> RequestResult<RequestBuilder> {
    let friend_ids = format!(
        "[{}]",
        ids.iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(client
        .request(Method::POST, "friends/diff.json")?
        .form(&DiffForm {
            friend_ids: &friend_ids,
        }))
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use url::Url;

    use crate::{
        agent::TalkApiAgent,
        client::{ApiClient, TalkHttpClient},
        config::Config,
        credential::Credential,
    };

    use super::{diff_request, FriendsDiff};

    fn client() -> ApiClient<'static> {
        ApiClient::new(
            Credential {
                access_token: "access",
                device_uuid: "device",
            },
            TalkHttpClient::new(
                Config {
                    language: "ko",
                    version: "26.7.2",
                    agent: TalkApiAgent::Android("16"),
                },
                Url::parse("https://katalk.kakao.com").unwrap(),
                Client::new(),
            ),
        )
    }

    #[test]
    fn diff_posts_only_the_current_friend_ids_field() {
        let request = diff_request(client(), &[10, 20, 30])
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            request.url().as_str(),
            "https://katalk.kakao.com/android/friends/diff.json"
        );
        assert_eq!(
            request.body().and_then(|body| body.as_bytes()).unwrap(),
            b"friend_ids=%5B10%2C+20%2C+30%5D"
        );
    }

    #[test]
    fn sparse_diff_response_uses_android_defaults() {
        let response: FriendsDiff = serde_json::from_str(r#"{"status":0}"#).unwrap();

        assert_eq!(response.total_count, 0);
        assert!(response.added_friends.is_empty());
        assert!(response.deleted_ids.is_empty());
    }
}
