pub mod auth;
pub mod constants;
pub mod friend;
pub mod profile;
pub mod protocol;

use std::{ops::Deref, time::Duration};

use reqwest::{redirect::Policy, Url};
use serde::Serialize;
use talk_api_internal::{
    client::{ApiClient, TalkHttpClient},
    credential::Credential,
    ApiError, ApiResult, RequestResult,
};
use tauri::{
    generate_handler,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State,
};

use crate::protocol::ProtocolProfile;

#[derive(Debug, Clone)]
#[repr(transparent)]
struct Client(reqwest::Client);

impl Deref for Client {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

type ClientState<'a> = State<'a, Client>;

pub async fn init<R: Runtime>() -> anyhow::Result<TauriPlugin<R>> {
    let client = Client(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(Policy::none())
            .https_only(true)
            .no_proxy()
            .build()?,
    );

    Ok(Builder::new("api")
        .setup(|app, _api| {
            app.manage(client);
            auth::init(app);

            Ok(())
        })
        .invoke_handler(generate_handler![
            auth::logon,
            auth::login,
            auth::logout,
            auth::auto_login,
            auth::poll_android_registration,
            auth::cancel_android_registration,
            auth::default_login_form,
            profile::me_profile,
            profile::friend_profile,
            friend::update_friends,
        ])
        .build())
}

fn create_api_client<'a>(
    client: &Client,
    access_token: &'a str,
    device_uuid: &'a str,
    profile: ProtocolProfile,
) -> ApiClient<'a> {
    ApiClient::new(
        Credential {
            device_uuid,
            access_token,
        },
        create_http_client(client, profile),
    )
}

fn create_http_client(client: &Client, profile: ProtocolProfile) -> TalkHttpClient<'static> {
    TalkHttpClient::new(
        profile.api_config(),
        Url::parse("https://katalk.kakao.com").unwrap(),
        client.0.clone(),
    )
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
enum Response<T> {
    Success(T),
    Failure(i32),
}

fn result_to_response<T>(result: ApiResult<T>) -> RequestResult<Response<T>> {
    match result {
        Ok(res) => Ok(Response::Success(res)),
        Err(ApiError::Status(status)) => Ok(Response::Failure(status)),
        Err(ApiError::Request(request)) => Err(request),
    }
}
