use anyhow::Context;
use kiwi_talk_result::TauriResult;
use serde::Serialize;
use talk_api_internal::{account::MoreSettings, profile::FriendInfo};

use crate::{
    auth::{CredentialExt, CredentialState},
    create_api_client, ClientState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,

    pub status_message: String,

    pub profile_url: String,
    pub background_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeProfile {
    pub nickname: String,

    pub uuid: Option<String>,
    pub uuid_searchable: bool,

    pub email: String,
    pub email_verified: bool,

    pub pstn_number: String,

    pub profile: Profile,
}

#[tauri::command]
pub(super) async fn me_profile(
    cred: CredentialState<'_>,
    client: ClientState<'_>,
) -> TauriResult<MeProfile> {
    let credential = cred.read().try_snapshot()?;

    let api = create_api_client(
        &client,
        credential.access_token.as_str(),
        &credential.device_uuid,
        credential.profile,
    );

    let more_settings = match credential.more_settings {
        Some(settings) => settings,
        None => {
            let settings = MoreSettings::request(api)
                .await
                .context("more_settings api call failed")?;

            if let Some(current) = cred
                .write()
                .as_mut()
                .filter(|current| current.user_id == credential.user_id)
            {
                current.more_settings = Some(settings.clone());
            }
            settings
        }
    };

    Ok(MeProfile {
        nickname: more_settings.nickname,

        uuid: more_settings.uuid,
        uuid_searchable: more_settings.uuid_searchable,

        email: more_settings.email_address,
        email_verified: more_settings.email_verified,

        pstn_number: more_settings.pstn_number,

        profile: Profile {
            id: credential.user_id.to_string(),
            status_message: more_settings.status_message,
            profile_url: more_settings.profile_image_url,
            background_url: String::new(),
        },
    })
}

#[tauri::command]
pub(super) async fn friend_profile(
    id: String,
    cred: CredentialState<'_>,
    client: ClientState<'_>,
) -> TauriResult<Profile> {
    let credential = cred.read().try_snapshot()?;

    let res = FriendInfo::request(
        create_api_client(
            &client,
            credential.access_token.as_str(),
            &credential.device_uuid,
            credential.profile,
        ),
        id.parse().context("invalid id")?,
    )
    .await
    .context("friend_info api call failed")?;

    Ok(Profile {
        id: res.profile.user_id.to_string(),
        status_message: res.profile.status_message,
        profile_url: res.profile.original_profile_image_url,
        background_url: res.profile.original_background_image_url,
    })
}
