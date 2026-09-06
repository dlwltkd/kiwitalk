//! Android subdevice authentication primitives.
//!
//! The wire format and default profile mirror KatokMCP's public Android
//! implementation at commit `56ae3d40022e6431c33e382b2ff854c5b46a8a14`
//! (2026-06-26):
//! <https://github.com/mwl313/KatokMCP/blob/56ae3d40022e6431c33e382b2ff854c5b46a8a14/packages/loco-engine/src/auth/android.ts>
//!
//! This module intentionally exposes one request per operation. It does not
//! automatically poll, sleep, cancel a challenge, print a passcode, or persist
//! credentials.

use std::fmt;

use reqwest::{
    header::{self, HeaderValue},
    Client as HttpClient, Method, RequestBuilder,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::{
    agent::TalkApiAgent,
    auth::{
        client::{AuthClient, Device},
        status::DEVICE_NOT_REGISTERED,
        xvc::default::AndroidSubXVCHasher,
        AccountForm, Login,
    },
    client::TalkHttpClient,
    config::Config,
    ApiError, ApiResult, RequestError, RequestResult,
};

pub const ANDROID_SUBDEVICE_MODEL: &str = "SM-X930";
pub const ANDROID_SUBDEVICE_APP_VERSION: &str = "25.9.2";
pub const ANDROID_SUBDEVICE_OS_VERSION: &str = "13";
pub const ANDROID_SUBDEVICE_API_LEVEL: &str = "33";
pub const ANDROID_SUBDEVICE_LANGUAGE: &str = "ko";

pub const ANDROID_SUBDEVICE_XVC_FIRST_SEED: &str = "BARD";
pub const ANDROID_SUBDEVICE_XVC_SECOND_SEED: &str = "DANTE";
pub const ANDROID_SUBDEVICE_XVC_THIRD_SEED: &str = "SIAN";

const FORM_CONTENT_TYPE: HeaderValue =
    HeaderValue::from_static("application/x-www-form-urlencoded");
const JSON_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/json; charset=utf-8");
const ALLOWLIST_XVC_IDENTITY: &str = "allowlist";
const MAX_AUTH_RESPONSE_SIZE: usize = 1024 * 1024;

/// A mutually consistent Android subdevice HTTP and X-VC profile.
///
/// The default constant below is an observed compatibility profile, not an
/// official Kakao API stability guarantee.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AndroidSubdeviceProfile {
    pub model: &'static str,
    pub app_version: &'static str,
    pub os_version: &'static str,
    pub api_level: &'static str,
    pub language: &'static str,
    pub xvc_first_seed: &'static str,
    pub xvc_second_seed: &'static str,
    pub xvc_third_seed: &'static str,
}

impl AndroidSubdeviceProfile {
    pub const fn config(self) -> Config<'static> {
        Config {
            language: self.language,
            version: self.app_version,
            agent: TalkApiAgent::Android(self.os_version),
        }
    }

    pub const fn xvc_hasher(self) -> AndroidSubXVCHasher<'static> {
        AndroidSubXVCHasher(
            self.xvc_first_seed,
            self.xvc_second_seed,
            self.xvc_third_seed,
        )
    }

    /// Build an Android auth client rooted at the supplied service URL.
    ///
    /// The caller owns the UUID lifecycle. This function does not generate,
    /// validate, log, or persist it.
    pub fn auth_client<'a>(
        self,
        device_uuid: &'a str,
        service_url: Url,
        http_client: HttpClient,
    ) -> AndroidAuthClient<'a> {
        let config: Config<'a> = self.config();
        let device = Device {
            name: self.model,
            model: Some(self.model),
            uuid: device_uuid,
        };
        let inner = TalkHttpClient::new(config, service_url, http_client);

        AndroidAuthClient {
            profile: self,
            inner: AuthClient::new(device, self.xvc_hasher(), inner),
        }
    }
}

/// KatokMCP's Android compatibility profile, verified from its June 2026
/// source snapshot.
pub const ANDROID_SUBDEVICE_PROFILE: AndroidSubdeviceProfile = AndroidSubdeviceProfile {
    model: ANDROID_SUBDEVICE_MODEL,
    app_version: ANDROID_SUBDEVICE_APP_VERSION,
    os_version: ANDROID_SUBDEVICE_OS_VERSION,
    api_level: ANDROID_SUBDEVICE_API_LEVEL,
    language: ANDROID_SUBDEVICE_LANGUAGE,
    xvc_first_seed: ANDROID_SUBDEVICE_XVC_FIRST_SEED,
    xvc_second_seed: ANDROID_SUBDEVICE_XVC_SECOND_SEED,
    xvc_third_seed: ANDROID_SUBDEVICE_XVC_THIRD_SEED,
};

/// An auth client tied to one Android compatibility profile and device UUID.
///
/// It deliberately has no `Debug` implementation because its internals can
/// derive X-VC values and contain a device UUID.
#[derive(Clone)]
pub struct AndroidAuthClient<'a> {
    profile: AndroidSubdeviceProfile,
    inner: AuthClient<'a, AndroidSubXVCHasher<'static>>,
}

impl AndroidAuthClient<'_> {
    fn request(
        self,
        method: Method,
        endpoint: &str,
        xvc_identity: &str,
    ) -> RequestResult<RequestBuilder> {
        Ok(self
            .inner
            .request(method, endpoint, xvc_identity)?
            .header(header::CONNECTION, "close"))
    }
}

/// A passcode challenge for the user to approve on their primary device.
///
/// `Debug` omits the passcode so accidental diagnostic output cannot reveal
/// it.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct AndroidPasscodeChallenge {
    pub passcode: String,
    #[serde(rename = "remainingSeconds")]
    pub remaining_seconds: u64,
}

impl fmt::Debug for AndroidPasscodeChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AndroidPasscodeChallenge")
            .field("remaining_seconds", &self.remaining_seconds)
            .finish_non_exhaustive()
    }
}

/// The result of exactly one registration-status request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidRegistrationPoll {
    Registered,
    Pending {
        remaining_seconds: u64,
        next_request_interval_seconds: u64,
    },
}

#[derive(Serialize)]
struct RegistrationBody<'a> {
    email: &'a str,
    password: &'a str,
    device: RegistrationDevice<'a>,
}

#[derive(Serialize)]
struct RegistrationDevice<'a> {
    uuid: &'a str,
}

impl<'a> RegistrationBody<'a> {
    fn new(device_uuid: &'a str, account: AccountForm<'a>) -> Self {
        Self {
            email: account.email,
            password: account.password,
            device: RegistrationDevice { uuid: device_uuid },
        }
    }
}

/// Check whether the profile's model is accepted for Android subdevice auth.
pub async fn check_allowlist(client: AndroidAuthClient<'_>) -> ApiResult<bool> {
    #[derive(Deserialize)]
    struct AllowlistResponse {
        #[serde(default)]
        allowlisted: bool,
    }

    let model = client.profile.model;
    let request = client
        .request(
            Method::GET,
            "account/allowlist.json",
            ALLOWLIST_XVC_IDENTITY,
        )?
        .header(header::CONTENT_TYPE, FORM_CONTENT_TYPE)
        .query(&[("model_name", model)]);
    let response: AllowlistResponse = read_http_structured_response(request).await?;

    Ok(response.allowlisted)
}

/// Generate a passcode challenge. Display and approval are left to the caller.
pub async fn generate_passcode(
    client: AndroidAuthClient<'_>,
    account: AccountForm<'_>,
) -> ApiResult<AndroidPasscodeChallenge> {
    #[derive(Serialize)]
    struct AndroidDevice<'a> {
        name: &'a str,
        uuid: &'a str,
        model: &'a str,
        #[serde(rename = "osVersion")]
        os_version: &'a str,
    }

    #[derive(Serialize)]
    struct GeneratePasscodeBody<'a> {
        email: &'a str,
        password: &'a str,
        permanent: bool,
        device: AndroidDevice<'a>,
    }

    let profile = client.profile;
    let device = client.inner.device;
    let body = GeneratePasscodeBody {
        email: account.email,
        password: account.password,
        permanent: true,
        device: AndroidDevice {
            name: device.name,
            uuid: device.uuid,
            model: device.model.unwrap_or(profile.model),
            os_version: profile.api_level,
        },
    };

    let body = serde_json::to_vec(&body)?;
    read_api_structured_response(
        client
            .request(
                Method::POST,
                "account/passcodeLogin/generate",
                account.email,
            )?
            .header(header::CONTENT_TYPE, JSON_CONTENT_TYPE)
            .body(body),
    )
    .await
}

/// Make one registration-status request.
///
/// A pending `-100` response is returned as data so the caller can choose its
/// own polling interval, deadline, cancellation, and retry policy. Any other
/// non-zero Kakao status remains an [`ApiError::Status`].
pub async fn poll_registration_once(
    client: AndroidAuthClient<'_>,
    account: AccountForm<'_>,
) -> ApiResult<AndroidRegistrationPoll> {
    #[derive(Deserialize)]
    struct StatusResponse {
        status: i32,
    }

    #[derive(Deserialize)]
    struct PendingResponse {
        #[serde(rename = "remainingSeconds")]
        remaining_seconds: u64,
        #[serde(rename = "nextRequestIntervalInSeconds")]
        next_request_interval_seconds: u64,
    }

    let body = RegistrationBody::new(client.inner.device.uuid, account);
    let body = serde_json::to_vec(&body)?;
    let bytes = read_http_response(
        client
            .request(
                Method::POST,
                "account/passcodeLogin/registerDevice",
                account.email,
            )?
            .header(header::CONTENT_TYPE, JSON_CONTENT_TYPE)
            .body(body),
    )
    .await?;
    let status: StatusResponse = serde_json::from_slice(&bytes)?;

    match status.status {
        0 => Ok(AndroidRegistrationPoll::Registered),
        DEVICE_NOT_REGISTERED => {
            let pending: PendingResponse = serde_json::from_slice(&bytes)?;
            Ok(AndroidRegistrationPoll::Pending {
                remaining_seconds: pending.remaining_seconds,
                next_request_interval_seconds: pending.next_request_interval_seconds,
            })
        }
        status => Err(ApiError::Status(status)),
    }
}

/// Cancel an outstanding passcode challenge.
///
/// Cleanup policy remains with the caller; none of the other primitives call
/// this automatically.
pub async fn cancel_registration(
    client: AndroidAuthClient<'_>,
    account: AccountForm<'_>,
) -> ApiResult<()> {
    let body = serde_json::to_vec(&RegistrationBody::new(client.inner.device.uuid, account))?;
    read_api_response(
        client
            .request(Method::POST, "account/passcodeLogin/cancel", account.email)?
            .header(header::CONTENT_TYPE, JSON_CONTENT_TYPE)
            .body(body),
    )
    .await?;

    Ok(())
}

/// Log in with an approved Android subdevice and return the token response.
pub async fn login(client: AndroidAuthClient<'_>, account: AccountForm<'_>) -> ApiResult<Login> {
    #[derive(Serialize)]
    struct LoginForm<'a> {
        password: &'a str,
        device_name: &'a str,
        forced: bool,
        permanent: bool,
        email: &'a str,
        device_uuid: &'a str,
    }

    let device = client.inner.device;
    let form = LoginForm {
        password: account.password,
        device_name: device.name,
        forced: false,
        permanent: true,
        email: account.email,
        device_uuid: device.uuid,
    };

    read_api_structured_response(
        client
            .request(Method::POST, "account/login.json", account.email)?
            .header(header::CONTENT_TYPE, FORM_CONTENT_TYPE)
            .form(&form),
    )
    .await
}

async fn read_http_structured_response<T: DeserializeOwned>(
    request: RequestBuilder,
) -> ApiResult<T> {
    Ok(serde_json::from_slice(&read_http_response(request).await?)?)
}

async fn read_api_structured_response<T: DeserializeOwned>(
    request: RequestBuilder,
) -> ApiResult<T> {
    Ok(serde_json::from_slice(&read_api_response(request).await?)?)
}

async fn read_api_response(request: RequestBuilder) -> ApiResult<Vec<u8>> {
    #[derive(Deserialize)]
    struct StatusResponse {
        status: i32,
    }

    let bytes = read_http_response(request).await?;
    let status: StatusResponse = serde_json::from_slice(&bytes)?;
    match status.status {
        0 => Ok(bytes),
        status => Err(ApiError::Status(status)),
    }
}

async fn read_http_response(request: RequestBuilder) -> ApiResult<Vec<u8>> {
    let mut response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(RequestError::HttpStatus(status).into());
    }

    if response
        .content_length()
        .is_some_and(|length| length > MAX_AUTH_RESPONSE_SIZE as u64)
    {
        return Err(RequestError::ResponseTooLarge {
            limit: MAX_AUTH_RESPONSE_SIZE,
        }
        .into());
    }

    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_AUTH_RESPONSE_SIZE as u64) as usize,
    );
    while let Some(chunk) = response.chunk().await? {
        let Some(size) = bytes.len().checked_add(chunk.len()) else {
            return Err(RequestError::ResponseTooLarge {
                limit: MAX_AUTH_RESPONSE_SIZE,
            }
            .into());
        };
        if size > MAX_AUTH_RESPONSE_SIZE {
            return Err(RequestError::ResponseTooLarge {
                limit: MAX_AUTH_RESPONSE_SIZE,
            }
            .into());
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };

    use super::*;
    use crate::{auth::xvc::XvcHasher, ApiError, RequestError};

    const EMAIL: &str = "test@example.com";
    const PASSWORD: &str = "synthetic-password";
    const UUID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    struct CapturedRequest {
        method: String,
        target: String,
        headers: HashMap<String, String>,
        body: Vec<u8>,
    }

    async fn mock_response(
        status: &'static str,
        response_body: &'static str,
    ) -> (Url, JoinHandle<CapturedRequest>) {
        mock_raw_response(
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            )
            .into_bytes(),
        )
        .await
    }

    async fn mock_response_with_declared_length(
        response_body: &'static str,
        content_length: usize,
    ) -> (Url, JoinHandle<CapturedRequest>) {
        mock_raw_response(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n{response_body}"
            )
            .into_bytes(),
        )
        .await
    }

    async fn mock_oversized_chunked_response() -> (Url, JoinHandle<CapturedRequest>) {
        let response_body = vec![b'x'; MAX_AUTH_RESPONSE_SIZE + 1];
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n",
            response_body.len()
        )
        .into_bytes();
        response.extend_from_slice(&response_body);
        response.extend_from_slice(b"\r\n0\r\n\r\n");
        mock_raw_response(response).await
    }

    async fn mock_raw_response(response: Vec<u8>) -> (Url, JoinHandle<CapturedRequest>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let header_end = loop {
                let mut chunk = [0_u8; 1024];
                let count = stream.read(&mut chunk).await.unwrap();
                assert!(count != 0);
                request.extend_from_slice(&chunk[..count]);
                if let Some(index) = request.windows(4).position(|value| value == b"\r\n\r\n") {
                    break index + 4;
                }
            };

            let header_text = std::str::from_utf8(&request[..header_end]).unwrap();
            let mut lines = header_text.split("\r\n");
            let request_line = lines.next().unwrap();
            let mut request_parts = request_line.split_whitespace();
            let method = request_parts.next().unwrap().to_owned();
            let target = request_parts.next().unwrap().to_owned();
            let mut headers = HashMap::new();
            for line in lines.filter(|line| !line.is_empty()) {
                let (name, value) = line.split_once(':').unwrap();
                headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
            }

            let content_length = headers
                .get("content-length")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            while request.len() - header_end < content_length {
                let mut chunk = [0_u8; 1024];
                let count = stream.read(&mut chunk).await.unwrap();
                assert!(count != 0);
                request.extend_from_slice(&chunk[..count]);
            }
            let body = request[header_end..header_end + content_length].to_vec();

            let _ = stream.write_all(&response).await;

            CapturedRequest {
                method,
                target,
                headers,
                body,
            }
        });

        (Url::parse(&format!("http://{address}/")).unwrap(), task)
    }

    fn client(url: Url) -> AndroidAuthClient<'static> {
        ANDROID_SUBDEVICE_PROFILE.auth_client(UUID, url, HttpClient::new())
    }

    fn account() -> AccountForm<'static> {
        AccountForm {
            email: EMAIL,
            password: PASSWORD,
        }
    }

    fn header_is(request: &CapturedRequest, name: &str, expected: &str) -> bool {
        request.headers.get(name).map(String::as_str) == Some(expected)
    }

    #[test]
    fn default_profile_matches_the_verified_android_xvc_vector() {
        let config = ANDROID_SUBDEVICE_PROFILE.config();
        let user_agent = config.get_user_agent();
        let digest = ANDROID_SUBDEVICE_PROFILE
            .xvc_hasher()
            .full_xvc_hash(UUID, &user_agent, EMAIL);
        let truncated = hex::encode(&digest[..8]);

        assert!(user_agent == "KT/25.9.2 An/13 ko");
        assert!(truncated == "5496390f221823b4");
    }

    #[tokio::test]
    async fn allowlist_uses_android_path_query_and_profile_headers() {
        let (url, captured) = mock_response("200 OK", r#"{"allowlisted":true}"#).await;

        let allowed = check_allowlist(client(url)).await.unwrap();
        let request = captured.await.unwrap();
        let allowlist_digest = ANDROID_SUBDEVICE_PROFILE.xvc_hasher().full_xvc_hash(
            UUID,
            "KT/25.9.2 An/13 ko",
            ALLOWLIST_XVC_IDENTITY,
        );
        let expected_xvc = hex::encode(&allowlist_digest[..8]);

        assert!(allowed);
        assert!(request.method == "GET");
        assert!(request.target == "/android/account/allowlist.json?model_name=SM-X930");
        assert!(request.body.is_empty());
        assert!(header_is(&request, "user-agent", "KT/25.9.2 An/13 ko"));
        assert!(header_is(&request, "a", "android/25.9.2/ko"));
        assert!(header_is(&request, "accept-language", "ko"));
        assert!(header_is(
            &request,
            "content-type",
            "application/x-www-form-urlencoded"
        ));
        assert!(header_is(&request, "connection", "close"));
        assert!(header_is(&request, "x-vc", &expected_xvc));
    }

    #[tokio::test]
    async fn passcode_generation_sends_the_nested_android_device_json() {
        let (url, captured) = mock_response(
            "200 OK",
            r#"{"status":0,"passcode":"654321","remainingSeconds":60}"#,
        )
        .await;

        let challenge = generate_passcode(client(url), account()).await.unwrap();
        let request = captured.await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        assert!(challenge.passcode == "654321");
        assert!(challenge.remaining_seconds == 60);
        assert!(request.method == "POST");
        assert!(request.target == "/android/account/passcodeLogin/generate");
        assert!(header_is(
            &request,
            "content-type",
            "application/json; charset=utf-8"
        ));
        assert!(header_is(&request, "x-vc", "5496390f221823b4"));
        assert!(body.get("email").and_then(serde_json::Value::as_str) == Some(EMAIL));
        assert!(body.get("password").and_then(serde_json::Value::as_str) == Some(PASSWORD));
        assert!(body.get("permanent").and_then(serde_json::Value::as_bool) == Some(true));
        let device = body
            .get("device")
            .and_then(serde_json::Value::as_object)
            .unwrap();
        assert!(device.get("name").and_then(serde_json::Value::as_str) == Some("SM-X930"));
        assert!(device.get("uuid").and_then(serde_json::Value::as_str) == Some(UUID));
        assert!(device.get("model").and_then(serde_json::Value::as_str) == Some("SM-X930"));
        assert!(device.get("osVersion").and_then(serde_json::Value::as_str) == Some("33"));

        let debug = format!("{challenge:?}");
        assert!(!debug.contains("654321"));
    }

    #[tokio::test]
    async fn one_registration_poll_returns_pending_metadata() {
        let (url, captured) = mock_response(
            "200 OK",
            r#"{"status":-100,"remainingSeconds":58,"nextRequestIntervalInSeconds":2}"#,
        )
        .await;

        let poll = poll_registration_once(client(url), account())
            .await
            .unwrap();
        let request = captured.await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        assert!(matches!(
            poll,
            AndroidRegistrationPoll::Pending {
                remaining_seconds: 58,
                next_request_interval_seconds: 2
            }
        ));
        assert!(request.method == "POST");
        assert!(request.target == "/android/account/passcodeLogin/registerDevice");
        assert!(body.get("email").and_then(serde_json::Value::as_str) == Some(EMAIL));
        assert!(body.get("password").and_then(serde_json::Value::as_str) == Some(PASSWORD));
        let device = body
            .get("device")
            .and_then(serde_json::Value::as_object)
            .unwrap();
        assert!(device.len() == 1);
        assert!(device.get("uuid").and_then(serde_json::Value::as_str) == Some(UUID));
    }

    #[tokio::test]
    async fn one_registration_poll_distinguishes_success_and_api_error() {
        let (success_url, success_request) = mock_response("200 OK", r#"{"status":0}"#).await;
        let success = poll_registration_once(client(success_url), account())
            .await
            .unwrap();
        success_request.await.unwrap();
        assert!(matches!(success, AndroidRegistrationPoll::Registered));

        let (error_url, error_request) = mock_response("200 OK", r#"{"status":-111}"#).await;
        let error = poll_registration_once(client(error_url), account())
            .await
            .unwrap_err();
        error_request.await.unwrap();
        assert!(matches!(error, ApiError::Status(-111)));
    }

    #[tokio::test]
    async fn cancellation_uses_the_registration_body_and_preserves_api_status() {
        let (url, captured) = mock_response("200 OK", r#"{"status":0}"#).await;

        cancel_registration(client(url), account()).await.unwrap();
        let request = captured.await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        assert!(request.method == "POST");
        assert!(request.target == "/android/account/passcodeLogin/cancel");
        assert!(header_is(
            &request,
            "content-type",
            "application/json; charset=utf-8"
        ));
        assert!(body.get("email").and_then(serde_json::Value::as_str) == Some(EMAIL));
        assert!(body.get("password").and_then(serde_json::Value::as_str) == Some(PASSWORD));
        let device = body
            .get("device")
            .and_then(serde_json::Value::as_object)
            .unwrap();
        assert!(device.len() == 1);
        assert!(device.get("uuid").and_then(serde_json::Value::as_str) == Some(UUID));

        let (error_url, error_request) = mock_response("200 OK", r#"{"status":-112}"#).await;
        let error = cancel_registration(client(error_url), account())
            .await
            .unwrap_err();
        error_request.await.unwrap();
        assert!(matches!(error, ApiError::Status(-112)));
    }

    #[tokio::test]
    async fn login_uses_android_form_and_parses_the_token_response() {
        let (url, captured) = mock_response(
            "200 OK",
            r#"{"status":0,"userId":9007199254740993123,"access_token":"synthetic-access","refresh_token":"synthetic-refresh","token_type":"bearer"}"#,
        )
        .await;

        let login = login(client(url), account()).await.unwrap();
        let request = captured.await.unwrap();
        let form: HashMap<_, _> = url::form_urlencoded::parse(&request.body)
            .into_owned()
            .collect();

        assert!(login.user_id == 9_007_199_254_740_993_123);
        assert!(login.access_token == "synthetic-access");
        assert!(login.refresh_token == "synthetic-refresh");
        assert!(login.token_type == "bearer");
        assert!(request.method == "POST");
        assert!(request.target == "/android/account/login.json");
        assert!(header_is(
            &request,
            "content-type",
            "application/x-www-form-urlencoded"
        ));
        assert!(form.get("email").map(String::as_str) == Some(EMAIL));
        assert!(form.get("password").map(String::as_str) == Some(PASSWORD));
        assert!(form.get("device_name").map(String::as_str) == Some("SM-X930"));
        assert!(form.get("device_uuid").map(String::as_str) == Some(UUID));
        assert!(form.get("forced").map(String::as_str) == Some("false"));
        assert!(form.get("permanent").map(String::as_str) == Some("true"));
        assert!(!form.contains_key("model_name"));
    }

    #[tokio::test]
    async fn non_success_http_status_is_preserved_without_body_parsing() {
        let (url, captured) = mock_response("429 Too Many Requests", "not JSON").await;

        let error = check_allowlist(client(url)).await.unwrap_err();
        captured.await.unwrap();

        assert!(matches!(
            error,
            ApiError::Request(RequestError::HttpStatus(
                reqwest::StatusCode::TOO_MANY_REQUESTS
            ))
        ));
    }

    #[tokio::test]
    async fn declared_oversized_response_is_rejected_without_exposing_its_body() {
        let (url, captured) = mock_response_with_declared_length(
            "synthetic-private-response",
            MAX_AUTH_RESPONSE_SIZE + 1,
        )
        .await;

        let error = check_allowlist(client(url)).await.unwrap_err();
        captured.await.unwrap();
        let message = error.to_string();

        assert!(matches!(
            error,
            ApiError::Request(RequestError::ResponseTooLarge {
                limit: MAX_AUTH_RESPONSE_SIZE
            })
        ));
        assert!(!message.contains("synthetic-private-response"));
    }

    #[tokio::test]
    async fn streamed_oversized_response_is_rejected_without_a_content_length() {
        let (url, captured) = mock_oversized_chunked_response().await;

        let error = check_allowlist(client(url)).await.unwrap_err();
        captured.await.unwrap();

        assert!(matches!(
            error,
            ApiError::Request(RequestError::ResponseTooLarge {
                limit: MAX_AUTH_RESPONSE_SIZE
            })
        ));
    }
}
