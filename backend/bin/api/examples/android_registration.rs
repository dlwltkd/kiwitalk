use std::{
    env,
    error::Error,
    fmt,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use kiwi_talk_system::derive_android_subdevice_uuid;
use reqwest::{redirect::Policy, Client, Url};
use talk_api_internal::{
    auth::{
        android::{
            cancel_registration, check_allowlist, generate_passcode, login, poll_registration_once,
            AndroidPasscodeChallenge, AndroidRegistrationPoll, ANDROID_SUBDEVICE_PROFILE,
        },
        status, AccountForm,
    },
    ApiError, RequestError,
};

const APP_IDENTIFIER: &str = "org.kiwitalk.kiwitalk";
const SERVICE_URL: &str = "https://katalk.kakao.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_POLL_ATTEMPTS: usize = 120;
const EXPIRY_TOLERANCE_SECONDS: u64 = 5;

struct Credentials {
    email: String,
    password: String,
}

#[derive(Debug)]
enum RunnerError {
    Usage,
    Setup(&'static str),
    Api(ApiError),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => formatter.write_str(
                "run with --confirm-registration to authorize one Android subdevice verification attempt",
            ),
            Self::Setup(message) => formatter.write_str(message),
            Self::Api(ApiError::Status(code)) => {
                write!(
                    formatter,
                    "Kakao rejected the request with status {code} ({})",
                    status_name(*code)
                )
            }
            Self::Api(ApiError::Request(RequestError::Reqwest(error)))
                if error.is_timeout() =>
            {
                formatter.write_str("the Kakao request timed out")
            }
            Self::Api(ApiError::Request(RequestError::Reqwest(error)))
                if error.is_connect() =>
            {
                formatter.write_str("could not establish a verified connection to Kakao")
            }
            Self::Api(ApiError::Request(RequestError::HttpStatus(http_status))) => {
                write!(
                    formatter,
                    "Kakao returned HTTP status {}",
                    http_status.as_u16()
                )
            }
            Self::Api(ApiError::Request(RequestError::Json(_))) => {
                formatter.write_str("Kakao returned an unexpected response schema")
            }
            Self::Api(ApiError::Request(RequestError::Url(_))) => {
                formatter.write_str("the Kakao service URL is invalid")
            }
            Self::Api(ApiError::Request(_)) => {
                formatter.write_str("the Kakao request failed before a status was received")
            }
        }
    }
}

impl Error for RunnerError {}

impl From<ApiError> for RunnerError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

fn repository_root() -> Result<&'static Path, RunnerError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or(RunnerError::Setup("could not locate the repository root"))
}

#[cfg(unix)]
fn ensure_private_regular_file(path: &Path, label: &'static str) -> Result<(), RunnerError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| RunnerError::Setup("could not inspect a required private file"))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(RunnerError::Setup(label));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(RunnerError::Setup(label));
    }

    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_regular_file(path: &Path, label: &'static str) -> Result<(), RunnerError> {
    if !path.is_file() {
        return Err(RunnerError::Setup(label));
    }

    Ok(())
}

fn load_credentials() -> Result<Credentials, RunnerError> {
    let path = repository_root()?.join(".env");
    ensure_private_regular_file(
        &path,
        "the repository .env must be a regular file readable only by its owner",
    )?;
    let entries = dotenvy::from_path_iter(path)
        .map_err(|_| RunnerError::Setup("could not load the repository .env"))?;
    let mut email = None;
    let mut password = None;

    for entry in entries {
        let (key, value) =
            entry.map_err(|_| RunnerError::Setup("could not parse the repository .env"))?;
        match key.as_str() {
            "kakaoemail" => email = Some(value),
            "kakaopw" => password = Some(value),
            _ => {}
        }
    }

    let email = email
        .filter(|value| !value.is_empty())
        .ok_or(RunnerError::Setup("kakaoemail is missing or empty in .env"))?;
    let password = password
        .filter(|value| !value.is_empty())
        .ok_or(RunnerError::Setup("kakaopw is missing or empty in .env"))?;

    Ok(Credentials { email, password })
}

fn device_uuid_path() -> Result<PathBuf, RunnerError> {
    let data_dir = if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else {
        PathBuf::from(env::var_os("HOME").ok_or(RunnerError::Setup("HOME is not set"))?)
            .join(".local/share")
    };

    Ok(data_dir.join(APP_IDENTIFIER).join("device_uuid"))
}

fn derive_android_device_uuid() -> Result<String, RunnerError> {
    let path = device_uuid_path()?;
    ensure_private_regular_file(
        &path,
        "the KiwiTalk device UUID must be a regular file readable only by its owner",
    )?;
    let bytes = std::fs::read(path)
        .map_err(|_| RunnerError::Setup("could not read the KiwiTalk device UUID"))?;
    let bytes: [u8; 64] = bytes.try_into().map_err(|_| {
        RunnerError::Setup("the KiwiTalk device UUID does not contain exactly 64 bytes")
    })?;
    let uuid = derive_android_subdevice_uuid(&bytes);
    if uuid.len() != 64
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RunnerError::Setup(
            "could not derive a valid Android device UUID",
        ));
    }

    Ok(uuid)
}

fn validate_challenge(challenge: &AndroidPasscodeChallenge) -> Result<(), RunnerError> {
    if !(4..=12).contains(&challenge.passcode.len())
        || !challenge.passcode.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(RunnerError::Setup(
            "Kakao returned an invalid verification-code format",
        ));
    }
    if challenge.remaining_seconds == 0 || challenge.remaining_seconds > 600 {
        return Err(RunnerError::Setup(
            "Kakao returned an invalid verification-code expiry",
        ));
    }

    Ok(())
}

fn status_name(code: i32) -> &'static str {
    match code {
        status::MISMATCH_PASSWORD => "mismatched password",
        status::EXCEED_LOGIN_LIMIT => "login limit exceeded",
        status::NOT_EXIST_ACCOUNT => "account not found",
        status::RESTRICTED_ACCOUNT | status::ACCOUNT_RESTRICTED => "account restricted",
        status::DEVICE_NOT_REGISTERED => "device registration required",
        status::ANOTHER_LOGON => "another client session is active",
        status::NEED_TERMS_AGREE => "terms agreement required",
        status::DENIED_DEVICE_MODEL => "device model denied",
        status::INVALID_STAGE_ERROR => "additional verification required",
        status::UPGRADE_REQUIRED => "client upgrade required",
        _ => "unknown status",
    }
}

async fn run() -> Result<(), RunnerError> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new("--confirm-registration"))
        || arguments.next().is_some()
    {
        return Err(RunnerError::Usage);
    }

    let credentials = load_credentials()?;
    let device_uuid = derive_android_device_uuid()?;
    let http = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .https_only(true)
        .no_proxy()
        .build()
        .map_err(|error| RunnerError::Api(RequestError::Reqwest(error).into()))?;
    let service_url = Url::parse(SERVICE_URL)
        .map_err(|_| RunnerError::Setup("the Kakao service URL is invalid"))?;
    let auth = ANDROID_SUBDEVICE_PROFILE.auth_client(&device_uuid, service_url, http);
    let account = AccountForm {
        email: &credentials.email,
        password: &credentials.password,
    };

    if !check_allowlist(auth.clone()).await? {
        return Err(RunnerError::Setup(
            "Kakao does not currently allow the Android compatibility model",
        ));
    }

    match login(auth.clone(), account).await {
        Ok(response) => {
            drop(response);
            println!("authentication accepted; tokens were not displayed or persisted");
            return Ok(());
        }
        Err(ApiError::Status(status::DEVICE_NOT_REGISTERED)) => {}
        Err(error) => return Err(error.into()),
    }

    let challenge = generate_passcode(auth.clone(), account).await?;
    let verification_result = async {
        validate_challenge(&challenge)?;
        println!("VERIFICATION_CODE={}", challenge.passcode);
        println!("EXPIRES_IN_SECONDS={}", challenge.remaining_seconds);
        println!("waiting for approval in the KakaoTalk mobile app...");
        io::stdout()
            .flush()
            .map_err(|_| RunnerError::Setup("could not display the verification code"))?;

        let deadline = Instant::now()
            + Duration::from_secs(
                challenge
                    .remaining_seconds
                    .saturating_add(EXPIRY_TOLERANCE_SECONDS),
            );

        for _ in 0..MAX_POLL_ATTEMPTS {
            if Instant::now() >= deadline {
                return Err(RunnerError::Setup("the verification code expired"));
            }

            match poll_registration_once(auth.clone(), account).await? {
                AndroidRegistrationPoll::Registered => return Ok(()),
                AndroidRegistrationPoll::Pending {
                    remaining_seconds,
                    next_request_interval_seconds,
                } => {
                    if remaining_seconds == 0
                        || remaining_seconds
                            > challenge
                                .remaining_seconds
                                .saturating_add(EXPIRY_TOLERANCE_SECONDS)
                    {
                        return Err(RunnerError::Setup(
                            "Kakao returned invalid verification timing data",
                        ));
                    }

                    let delay_seconds = next_request_interval_seconds.max(1).min(remaining_seconds);
                    let delay = Duration::from_secs(delay_seconds);
                    if Instant::now()
                        .checked_add(delay)
                        .is_none_or(|wake_at| wake_at > deadline)
                    {
                        return Err(RunnerError::Setup("the verification code expired"));
                    }
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(RunnerError::Setup(
            "verification polling exceeded its attempt limit",
        ))
    }
    .await;

    if let Err(error) = verification_result {
        let _ = cancel_registration(auth.clone(), account).await;
        return Err(error);
    }

    let response = login(auth, account).await?;
    drop(response);
    println!("authentication accepted; tokens were not displayed or persisted");

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_validation_rejects_terminal_control_text() {
        let challenge = AndroidPasscodeChallenge {
            passcode: String::from("12\u{1b}[31m"),
            remaining_seconds: 60,
        };

        assert!(validate_challenge(&challenge).is_err());
    }

    #[test]
    fn android_uuid_derivation_is_domain_separated_and_stable() {
        let first = derive_android_subdevice_uuid(&[7_u8; 64]);
        let second = derive_android_subdevice_uuid(&[7_u8; 64]);

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
