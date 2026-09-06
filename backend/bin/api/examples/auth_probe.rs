use std::{env, error::Error, path::Path, path::PathBuf, process::ExitCode, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use kiwi_talk_api::constants::{TALK_AGENT, TALK_VERSION, XVC_HASHER};
use reqwest::{redirect::Policy, Client, Url};
use talk_api_internal::{
    auth::{
        client::{AuthClient, Device},
        status,
        xvc::default::Win32XVCHasher,
        AccountForm, Login,
    },
    client::TalkHttpClient,
    config::Config,
    ApiError, RequestError,
};

const APP_IDENTIFIER: &str = "org.kiwitalk.kiwitalk";

struct Credentials {
    email: String,
    password: String,
}

fn load_credentials() -> Result<Credentials, Box<dyn Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or("could not locate the repository root")?;
    let entries = dotenvy::from_path_iter(repository.join(".env"))
        .map_err(|_| "could not load the repository .env")?;
    let mut email = None;
    let mut password = None;

    for entry in entries {
        let (key, value) = entry.map_err(|_| "could not parse the repository .env")?;
        match key.as_str() {
            "kakaoemail" => email = Some(value),
            "kakaopw" => password = Some(value),
            _ => {}
        }
    }

    let email = email
        .filter(|value| !value.is_empty())
        .ok_or("kakaoemail is missing or empty in .env")?;
    let password = password
        .filter(|value| !value.is_empty())
        .ok_or("kakaopw is missing or empty in .env")?;

    Ok(Credentials { email, password })
}

fn device_uuid_path() -> Result<PathBuf, Box<dyn Error>> {
    let data_dir = if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else {
        PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?).join(".local/share")
    };

    Ok(data_dir.join(APP_IDENTIFIER).join("device_uuid"))
}

fn read_device_uuid() -> Result<String, Box<dyn Error>> {
    let bytes = std::fs::read(device_uuid_path()?)?;
    if bytes.len() != 64 {
        return Err(format!(
            "KiwiTalk device UUID must contain 64 bytes, found {}",
            bytes.len()
        )
        .into());
    }

    Ok(STANDARD.encode(bytes))
}

fn status_name(code: i32) -> &'static str {
    match code {
        status::MISMATCH_PASSWORD => "mismatched password",
        status::EXCEED_LOGIN_LIMIT => "login limit exceeded",
        status::NOT_EXIST_ACCOUNT => "account not found",
        status::RESTRICTED_ACCOUNT | status::ACCOUNT_RESTRICTED => "account restricted",
        status::DEVICE_NOT_REGISTERED => "device registration required",
        status::ANOTHER_LOGON => "another desktop session is active",
        status::NEED_TERMS_AGREE => "terms agreement required",
        status::DENIED_DEVICE_MODEL => "device model denied",
        status::INVALID_STAGE_ERROR => "additional verification required",
        status::UPGRADE_REQUIRED => "client upgrade required",
        _ => "unknown status",
    }
}

fn report_error(error: ApiError) {
    match error {
        ApiError::Status(code) => {
            println!("auth rejected: status {code} ({})", status_name(code));
        }
        ApiError::Request(RequestError::Reqwest(error))
            if error.is_connect() && error.is_timeout() =>
        {
            println!("auth transport failure: TCP/TLS connection timed out");
        }
        ApiError::Request(RequestError::Reqwest(error)) if error.is_timeout() => {
            println!("auth transport failure: request timed out");
        }
        ApiError::Request(RequestError::Reqwest(error)) if error.is_connect() => {
            println!("auth transport failure: could not connect to Kakao");
        }
        ApiError::Request(RequestError::Reqwest(error)) => {
            println!(
                "auth HTTP failure: {}",
                error
                    .status()
                    .map(|status| status.as_u16().to_string())
                    .unwrap_or_else(|| String::from("no status"))
            );
        }
        ApiError::Request(RequestError::HttpStatus(status)) => {
            println!("auth HTTP failure: status {}", status.as_u16());
        }
        ApiError::Request(RequestError::Json(_)) => {
            println!("auth response failure: Kakao returned an unexpected response schema");
        }
        ApiError::Request(RequestError::Url(_)) => {
            println!("auth setup failure: invalid endpoint URL");
        }
        ApiError::Request(_) => {
            println!("auth request failed before a status could be read");
        }
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    let credentials = load_credentials()?;
    let device_uuid = read_device_uuid()?;
    let device_name = hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| String::from("Unknown"));

    let http = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .https_only(true)
        .no_proxy()
        .build()?;
    let auth = AuthClient::new(
        Device {
            name: &device_name,
            model: None,
            uuid: &device_uuid,
        },
        Win32XVCHasher(XVC_HASHER.0, XVC_HASHER.1),
        TalkHttpClient::new(
            Config {
                language: "ko",
                version: TALK_VERSION,
                agent: TALK_AGENT,
            },
            Url::parse("https://katalk.kakao.com")?,
            http,
        ),
    );

    println!("probing Kakao authentication with desktop version {TALK_VERSION}");
    match Login::request_with_account(
        auth,
        AccountForm {
            email: &credentials.email,
            password: &credentials.password,
        },
        false,
    )
    .await
    {
        Ok(_) => {
            println!("auth accepted; credentials and tokens were intentionally not displayed");
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            report_error(error);
            Ok(ExitCode::FAILURE)
        }
    }
}
