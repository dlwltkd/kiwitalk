mod account;

use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context};
use kiwi_talk_result::TauriResult;
use kiwi_talk_system::get_system_info;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use talk_api_internal::{
    account::MoreSettings,
    auth::{
        android::{
            cancel_registration, check_allowlist, generate_passcode, login as android_login,
            poll_registration_once, refresh_access_token, AndroidAuthClient,
            AndroidPasscodeChallenge, AndroidRegistrationPoll,
        },
        status, AccountForm, Login,
    },
    ApiError,
};
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    create_api_client,
    protocol::{ProtocolProfile, ACTIVE_PROTOCOL_PROFILE},
    result_to_response, Client, ClientState, Response,
};

use self::account::{SavedAccount, SavedSession};

const REGISTRATION_EXPIRY_TOLERANCE_SECONDS: u64 = 5;

#[tauri::command]
pub(super) fn logon(state: CredentialState<'_>) -> bool {
    state.read().is_some()
}

#[tauri::command(async)]
pub(super) async fn default_login_form() -> Result<LoginDetailForm, ()> {
    let saved_account = account::read().await.unwrap_or_default();
    let saved_session = account::read_session().await.unwrap_or_default();

    Ok(match saved_account {
        Some(data) => {
            let current_uuid = get_system_info()
                .device
                .device_uuid
                .android_subdevice_uuid();
            let auto_login = saved_session.as_ref().is_some_and(|session| {
                session.email == data.email && session.device_uuid == current_uuid
            });

            LoginDetailForm {
                profile: data.profile,
                name: data.name,
                email: data.email,
                password: String::new(),
                save_email: true,
                auto_login,
            }
        }
        None => Default::default(),
    })
}

#[tauri::command(async)]
pub(super) async fn login(
    form: LoginForm,
    cred: CredentialState<'_>,
    pending: PendingRegistrationState<'_>,
    client: ClientState<'_>,
) -> TauriResult<Response<LoginOutcome>> {
    let profile = ACTIVE_PROTOCOL_PROFILE;
    let device_uuid = get_system_info()
        .device
        .device_uuid
        .android_subdevice_uuid();
    let mut pending_slot = pending.pending.lock().await;

    if let Some(previous) = pending_slot.take() {
        let _ = cancel_pending(&client, &previous).await;
    }

    let auth = create_auth_client(&client, &device_uuid, profile);
    let allowed = match result_to_response(check_allowlist(auth.clone()).await)
        .context("Android device allowlist request failed")?
    {
        Response::Success(allowed) => allowed,
        Response::Failure(code) => return Ok(Response::Failure(code)),
    };
    if !allowed {
        return Ok(Response::Failure(status::DENIED_DEVICE_MODEL));
    }

    let account = form.account();
    match android_login(auth.clone(), account).await {
        Ok(login) => {
            drop(pending_slot);
            finish_login(&client, &cred, &form, device_uuid, profile, login).await;
            Ok(Response::Success(LoginOutcome::Authenticated))
        }
        Err(ApiError::Status(status::DEVICE_NOT_REGISTERED)) => {
            let challenge = match result_to_response(generate_passcode(auth, account).await)
                .context("Android passcode generation failed")?
            {
                Response::Success(challenge) => challenge,
                Response::Failure(code) => return Ok(Response::Failure(code)),
            };
            validate_challenge(&challenge)?;

            let id = pending.next_id.fetch_add(1, Ordering::Relaxed);
            let expires_at = Instant::now()
                + Duration::from_secs(
                    challenge
                        .remaining_seconds
                        .saturating_add(REGISTRATION_EXPIRY_TOLERANCE_SECONDS),
                );
            let response = VerificationChallenge {
                id: id.to_string(),
                passcode: challenge.passcode,
                remaining_seconds: challenge.remaining_seconds,
            };
            *pending_slot = Some(PendingRegistration {
                id,
                form,
                device_uuid,
                profile,
                initial_remaining_seconds: response.remaining_seconds,
                expires_at,
            });

            Ok(Response::Success(LoginOutcome::VerificationRequired(
                response,
            )))
        }
        Err(ApiError::Status(code)) => Ok(Response::Failure(code)),
        Err(ApiError::Request(error)) => Err(error.into()),
    }
}

#[tauri::command(async)]
pub(super) async fn poll_android_registration(
    id: String,
    cred: CredentialState<'_>,
    pending: PendingRegistrationState<'_>,
    client: ClientState<'_>,
) -> TauriResult<Response<RegistrationOutcome>> {
    let Ok(id) = id.parse::<u64>() else {
        return Ok(Response::Success(RegistrationOutcome::Stale));
    };
    let mut pending_slot = pending.pending.lock().await;
    let Some(registration) = pending_slot.as_ref() else {
        return Ok(Response::Success(RegistrationOutcome::Stale));
    };
    if registration.id != id {
        return Ok(Response::Success(RegistrationOutcome::Stale));
    }
    if Instant::now() >= registration.expires_at {
        let registration = pending_slot.take().unwrap();
        let _ = cancel_pending(&client, &registration).await;
        return Ok(Response::Success(RegistrationOutcome::Expired));
    }

    let auth = create_auth_client(&client, &registration.device_uuid, registration.profile);
    match poll_registration_once(auth.clone(), registration.form.account()).await {
        Ok(AndroidRegistrationPoll::Pending {
            remaining_seconds,
            next_request_interval_seconds,
        }) => {
            if remaining_seconds == 0
                || remaining_seconds
                    > registration
                        .initial_remaining_seconds
                        .saturating_add(REGISTRATION_EXPIRY_TOLERANCE_SECONDS)
            {
                let registration = pending_slot.take().unwrap();
                let _ = cancel_pending(&client, &registration).await;
                return Ok(Response::Success(RegistrationOutcome::Expired));
            }

            Ok(Response::Success(RegistrationOutcome::Pending {
                remaining_seconds,
                next_request_interval_seconds: next_request_interval_seconds
                    .max(1)
                    .min(remaining_seconds),
            }))
        }
        Ok(AndroidRegistrationPoll::Registered) => {
            let login = match android_login(auth, registration.form.account()).await {
                Ok(login) => login,
                Err(ApiError::Status(code)) => {
                    pending_slot.take();
                    return Ok(Response::Failure(code));
                }
                Err(ApiError::Request(error)) => return Err(error.into()),
            };

            let registration = pending_slot.take().unwrap();
            drop(pending_slot);
            finish_login(
                &client,
                &cred,
                &registration.form,
                registration.device_uuid.clone(),
                registration.profile,
                login,
            )
            .await;

            Ok(Response::Success(RegistrationOutcome::Authenticated))
        }
        Err(ApiError::Status(code)) => {
            let registration = pending_slot.take().unwrap();
            let _ = cancel_pending(&client, &registration).await;
            Ok(Response::Failure(code))
        }
        Err(ApiError::Request(error)) => Err(error.into()),
    }
}

#[tauri::command(async)]
pub(super) async fn cancel_android_registration(
    id: String,
    pending: PendingRegistrationState<'_>,
    client: ClientState<'_>,
) -> TauriResult<Response<bool>> {
    let Ok(id) = id.parse::<u64>() else {
        return Ok(Response::Success(false));
    };
    let mut pending_slot = pending.pending.lock().await;
    let Some(registration) = pending_slot.as_ref() else {
        return Ok(Response::Success(false));
    };
    if registration.id != id {
        return Ok(Response::Success(false));
    }

    let registration = pending_slot.take().unwrap();
    let _ = cancel_pending(&client, &registration).await;
    Ok(Response::Success(true))
}

#[tauri::command(async)]
pub(super) async fn logout(
    cred: CredentialState<'_>,
    pending: PendingRegistrationState<'_>,
    client: ClientState<'_>,
) -> TauriResult<bool> {
    if let Some(registration) = pending.pending.lock().await.take() {
        let _ = cancel_pending(&client, &registration).await;
    }

    account::write_session(None)
        .await
        .context("could not clear the saved Android login session")?;

    Ok(cred.write().take().is_some())
}

#[tauri::command(async)]
pub(super) async fn auto_login(
    cred: CredentialState<'_>,
    client: ClientState<'_>,
) -> TauriResult<Response<bool>> {
    if cred.read().is_some() {
        return Ok(Response::Success(true));
    }

    let Some(mut session) = account::read_session()
        .await
        .context("could not read the saved Android login session")?
    else {
        return Ok(Response::Success(false));
    };

    let device_uuid = get_system_info()
        .device
        .device_uuid
        .android_subdevice_uuid();
    if session.device_uuid != device_uuid {
        account::write_session(None)
            .await
            .context("could not clear a login session for another device")?;
        return Ok(Response::Success(false));
    }

    let profile = ACTIVE_PROTOCOL_PROFILE;
    let auth = create_auth_client(&client, &device_uuid, profile);
    let refreshed = match result_to_response(
        refresh_access_token(auth, &session.email, &session.refresh_token).await,
    )
    .context("Android session refresh failed")?
    {
        Response::Success(refreshed) => refreshed,
        Response::Failure(code) => return Ok(Response::Failure(code)),
    };

    let user_id = session.user_id;
    session.refresh_token.zeroize();
    session.refresh_token = refreshed.refresh_token.clone();
    account::write_session(Some(session))
        .await
        .context("could not persist the rotated Android login session")?;

    *cred.write() = Some(Credential {
        user_id,
        device_uuid,
        profile,
        access_token: Zeroizing::new(refreshed.access_token),
        _refresh_token: Zeroizing::new(refreshed.refresh_token),
        more_settings: None,
    });

    Ok(Response::Success(true))
}

pub(super) fn init(app: &AppHandle<impl Runtime>) {
    app.manage(CredentialStateSlot::new(None));
    app.manage(PendingRegistrationSlot::new());
}

async fn finish_login(
    client: &Client,
    cred: &CredentialState<'_>,
    form: &LoginForm,
    device_uuid: String,
    profile: ProtocolProfile,
    login: Login,
) {
    let account_email = if login.auto_login_account_id.is_empty() {
        form.email.as_str()
    } else {
        login.auto_login_account_id.as_str()
    };
    let save_email = form.save_email || form.auto_login;
    let cached_settings = if save_email {
        let api = create_api_client(client, &login.access_token, &device_uuid, profile);
        match MoreSettings::request(api).await {
            Ok(settings) => Some(settings),
            Err(_) => {
                log::warn!(
                    "profile lookup failed after successful Android login; continuing without cached profile"
                );
                None
            }
        }
    } else {
        None
    };

    let saved = save_email.then(|| SavedAccount {
        profile: cached_settings
            .as_ref()
            .map(|settings| settings.profile_image_url.clone())
            .unwrap_or_default(),
        name: cached_settings
            .as_ref()
            .map(|settings| settings.nickname.clone())
            .unwrap_or_default(),
        email: account_email.to_owned(),
        token: None,
    });
    if account::write(saved).await.is_err() {
        log::warn!("could not update saved account metadata after Android login");
    }

    let saved_session = form.auto_login.then(|| SavedSession {
        email: account_email.to_owned(),
        user_id: login.user_id,
        device_uuid: device_uuid.clone(),
        refresh_token: login.refresh_token.clone(),
    });
    if account::write_session(saved_session).await.is_err() {
        log::warn!("could not update the saved Android login session");
    }

    *cred.write() = Some(Credential {
        user_id: login.user_id,
        device_uuid,
        profile,
        access_token: Zeroizing::new(login.access_token),
        _refresh_token: Zeroizing::new(login.refresh_token),
        more_settings: cached_settings,
    });
}

async fn cancel_pending(client: &Client, pending: &PendingRegistration) -> Result<(), ApiError> {
    cancel_registration(
        create_auth_client(client, &pending.device_uuid, pending.profile),
        pending.form.account(),
    )
    .await
}

fn create_auth_client<'a>(
    client: &Client,
    device_uuid: &'a str,
    profile: ProtocolProfile,
) -> AndroidAuthClient<'a> {
    profile.api_profile().auth_client(
        device_uuid,
        reqwest::Url::parse("https://katalk.kakao.com").unwrap(),
        client.0.clone(),
    )
}

fn validate_challenge(challenge: &AndroidPasscodeChallenge) -> anyhow::Result<()> {
    if !(4..=12).contains(&challenge.passcode.len())
        || !challenge.passcode.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(anyhow!(
            "Kakao returned an invalid verification-code format"
        ));
    }
    if challenge.remaining_seconds == 0 || challenge.remaining_seconds > 600 {
        return Err(anyhow!(
            "Kakao returned an invalid verification-code expiry"
        ));
    }

    Ok(())
}

pub struct Credential {
    pub(crate) user_id: u64,
    device_uuid: String,
    profile: ProtocolProfile,
    access_token: Zeroizing<String>,
    _refresh_token: Zeroizing<String>,
    pub(crate) more_settings: Option<MoreSettings>,
}

impl fmt::Debug for Credential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credential")
            .field("user_id", &self.user_id)
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

pub struct CredentialSnapshot {
    pub user_id: u64,
    pub device_uuid: String,
    pub profile: ProtocolProfile,
    pub access_token: Zeroizing<String>,
    pub more_settings: Option<MoreSettings>,
}

impl Credential {
    pub fn snapshot(&self) -> CredentialSnapshot {
        CredentialSnapshot {
            user_id: self.user_id,
            device_uuid: self.device_uuid.clone(),
            profile: self.profile,
            access_token: self.access_token.clone(),
            more_settings: self.more_settings.clone(),
        }
    }
}

#[easy_ext::ext(CredentialExt)]
pub(crate) impl Option<Credential> {
    fn try_snapshot(&self) -> anyhow::Result<CredentialSnapshot> {
        self.as_ref()
            .map(Credential::snapshot)
            .ok_or_else(|| anyhow!("not logon"))
    }
}

pub type CredentialStateSlot = RwLock<Option<Credential>>;
pub type CredentialState<'a> = tauri::State<'a, CredentialStateSlot>;

struct PendingRegistration {
    id: u64,
    form: LoginForm,
    device_uuid: String,
    profile: ProtocolProfile,
    initial_remaining_seconds: u64,
    expires_at: Instant,
}

pub struct PendingRegistrationSlot {
    next_id: AtomicU64,
    pending: Mutex<Option<PendingRegistration>>,
}

impl PendingRegistrationSlot {
    const fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            pending: Mutex::const_new(None),
        }
    }
}

type PendingRegistrationState<'a> = tauri::State<'a, PendingRegistrationSlot>;

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum LoginOutcome {
    Authenticated,
    VerificationRequired(VerificationChallenge),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationChallenge {
    id: String,
    passcode: String,
    remaining_seconds: u64,
}

impl fmt::Debug for VerificationChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerificationChallenge")
            .field("id", &self.id)
            .field("remaining_seconds", &self.remaining_seconds)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum RegistrationOutcome {
    Pending {
        #[serde(rename = "remainingSeconds")]
        remaining_seconds: u64,
        #[serde(rename = "nextRequestIntervalSeconds")]
        next_request_interval_seconds: u64,
    },
    Authenticated,
    Expired,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginDetailForm {
    pub profile: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub save_email: bool,
    pub auto_login: bool,
}

impl Default for LoginDetailForm {
    fn default() -> Self {
        Self {
            profile: Default::default(),
            name: Default::default(),
            email: Default::default(),
            password: Default::default(),
            save_email: true,
            auto_login: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub save_email: bool,
    pub auto_login: bool,
}

impl LoginForm {
    fn account(&self) -> AccountForm<'_> {
        AccountForm {
            email: &self.email,
            password: &self.password,
        }
    }
}

impl fmt::Debug for LoginForm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginForm")
            .field("email", &self.email)
            .field("save_email", &self.save_email)
            .field("auto_login", &self.auto_login)
            .finish_non_exhaustive()
    }
}

impl Drop for LoginForm {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_form_debug_omits_password() {
        let form = LoginForm {
            email: String::from("test@example.com"),
            password: String::from("synthetic-password"),
            save_email: true,
            auto_login: false,
        };

        let debug = format!("{form:?}");
        assert!(debug.contains("test@example.com"));
        assert!(!debug.contains("synthetic-password"));
    }

    #[test]
    fn challenge_validation_rejects_control_text() {
        let challenge = AndroidPasscodeChallenge {
            passcode: String::from("12\u{1b}[31m"),
            remaining_seconds: 60,
        };

        assert!(validate_challenge(&challenge).is_err());
    }
}
