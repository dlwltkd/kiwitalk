pub mod android;
pub mod client;
pub mod status;
pub mod xvc;

use std::fmt;

use reqwest::Method;

use crate::{read_response, read_structured_response, ApiResult};

use self::{
    client::{AuthClient, Device},
    xvc::XvcHasher,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize)]
pub struct AccountForm<'a> {
    pub email: &'a str,
    pub password: &'a str,
}

impl fmt::Debug for AccountForm<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountForm")
            .field("email", &self.email)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Deserialize)]
pub struct Login {
    #[serde(rename = "userId")]
    pub user_id: u64,

    #[serde(rename = "countryIso")]
    #[serde(default)]
    pub country_iso: String,
    #[serde(rename = "countryCode")]
    #[serde(default)]
    pub country_code: String,

    #[serde(rename = "accountId")]
    #[serde(default)]
    pub account_id: u64,

    // pub server_time: u64,

    // #[serde(rename = "resetUserData")]
    // pub reset_user_data: bool,
    // pub story_url: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,

    #[serde(rename = "autoLoginAccountId")]
    #[serde(default)]
    pub auto_login_account_id: String,
    #[serde(rename = "displayAccountId")]
    #[serde(default)]
    pub display_account_id: String,

    #[serde(rename = "mainDeviceAgentName")]
    #[serde(default)]
    pub main_device_agent_name: String,
    #[serde(rename = "mainDeviceAppVersion")]
    #[serde(default)]
    pub main_device_app_version: String,
}

impl fmt::Debug for Login {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Login")
            .field("user_id", &self.user_id)
            .field("country_iso", &self.country_iso)
            .field("country_code", &self.country_code)
            .field("account_id", &self.account_id)
            .field("auto_login_account_id", &self.auto_login_account_id)
            .field("display_account_id", &self.display_account_id)
            .field("main_device_agent_name", &self.main_device_agent_name)
            .field("main_device_app_version", &self.main_device_app_version)
            .finish_non_exhaustive()
    }
}

impl Login {
    pub async fn request_with_account(
        client: AuthClient<'_, impl XvcHasher>,
        account: AccountForm<'_>,
        forced: bool,
    ) -> ApiResult<Self> {
        #[derive(Serialize)]
        struct Form<'a> {
            #[serde(flatten)]
            device: Device<'a>,

            #[serde(flatten)]
            account: AccountForm<'a>,
            forced: bool,
        }

        let form = Form {
            device: client.device,
            account,
            forced,
        };

        read_structured_response(
            client
                .request(Method::POST, "account/login.json", account.email)?
                .form(&form),
        )
        .await
    }

    pub async fn request_with_token(
        client: AuthClient<'_, impl XvcHasher>,
        email: &str,
        token: &str,
        forced: bool,
        locked: bool,
    ) -> ApiResult<Self> {
        #[derive(Serialize)]
        struct Form<'a> {
            #[serde(flatten)]
            device: Device<'a>,

            email: &'a str,
            password: &'a str,
            auto_login: bool,
            autowithlock: bool,
            forced: bool,
        }

        let form = Form {
            device: client.device,
            email,
            password: token,
            auto_login: true,
            autowithlock: locked,
            forced,
        };

        read_structured_response(
            client
                .request(Method::POST, "account/login.json", email)?
                .form(&form),
        )
        .await
    }
}

pub async fn request_passcode(
    client: AuthClient<'_, impl XvcHasher>,
    account: AccountForm<'_>,
) -> ApiResult<()> {
    #[derive(Serialize)]
    struct Form<'a> {
        #[serde(flatten)]
        device: Device<'a>,

        #[serde(flatten)]
        account: AccountForm<'a>,
    }

    let form = Form {
        device: client.device,
        account,
    };

    read_response(
        client
            .request(Method::POST, "account/request_passcode.json", account.email)?
            .form(&form),
    )
    .await?;

    Ok(())
}

pub async fn register_device(
    client: AuthClient<'_, impl XvcHasher>,
    account: AccountForm<'_>,
    passcode: &str,
    permanent: bool,
) -> ApiResult<()> {
    #[derive(Serialize)]
    struct Form<'a> {
        #[serde(flatten)]
        device: Device<'a>,

        #[serde(flatten)]
        account: AccountForm<'a>,

        passcode: &'a str,
        permanent: bool,
    }

    let form = Form {
        device: client.device,
        account,
        passcode,
        permanent,
    };

    read_response(
        client
            .request(Method::POST, "account/register_device.json", account.email)?
            .form(&form),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AccountForm, Login};

    #[test]
    fn login_accepts_response_without_unused_legacy_metadata() {
        let login: Login = serde_json::from_value(serde_json::json!({
            "userId": 42,
            "access_token": "synthetic-access",
            "refresh_token": "synthetic-refresh",
            "token_type": "Bearer"
        }))
        .unwrap();

        assert_eq!(login.user_id, 42);
        assert!(login.auto_login_account_id.is_empty());
        assert!(login.main_device_app_version.is_empty());

        let debug = format!("{login:?}");
        assert!(!debug.contains("synthetic-access"));
        assert!(!debug.contains("synthetic-refresh"));
    }

    #[test]
    fn account_debug_omits_the_password() {
        let account = AccountForm {
            email: "test@example.com",
            password: "synthetic-password",
        };

        assert!(!format!("{account:?}").contains("synthetic-password"));
    }
}
