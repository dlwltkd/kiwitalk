use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayUser {
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(rename = "nickName")]
    pub nickname: String,

    #[serde(rename = "profileImageUrl")]
    pub profile_image_url: Option<String>,

    #[serde(
        default,
        rename = "countryIso",
        deserialize_with = "deserialize_null_default"
    )]
    pub country_iso: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct User {
    #[serde(rename = "userId")]
    pub user_id: i64,

    #[serde(rename = "nickName")]
    pub nickname: String,

    #[serde(rename = "countryIso")]
    pub country_iso: String,

    #[serde(rename = "profileImageUrl")]
    pub profile_image_url: String,

    #[serde(rename = "fullProfileImageUrl")]
    pub full_profile_image_url: String,

    #[serde(rename = "originalProfileImageUrl")]
    pub original_profile_image_url: String,

    /// See UserType for types.
    #[serde(rename = "type")]
    pub user_type: i32,

    #[serde(rename = "accountId")]
    pub account_id: i64,

    #[serde(rename = "linkedServices")]
    pub linked_services: String,

    #[serde(rename = "statusMessage")]
    pub status_message: String,
    pub suspended: bool,

    #[serde(skip)]
    presence: UserFieldPresence,
}

impl User {
    pub fn country_iso_if_present(&self) -> Option<&str> {
        self.presence.country_iso.then_some(&self.country_iso)
    }

    pub fn profile_image_url_if_present(&self) -> Option<&str> {
        self.presence
            .profile_image_url
            .then_some(&self.profile_image_url)
    }

    pub fn full_profile_image_url_if_present(&self) -> Option<&str> {
        self.presence
            .full_profile_image_url
            .then_some(&self.full_profile_image_url)
    }

    pub fn original_profile_image_url_if_present(&self) -> Option<&str> {
        self.presence
            .original_profile_image_url
            .then_some(&self.original_profile_image_url)
    }

    pub fn account_id_if_present(&self) -> Option<i64> {
        self.presence.account_id.then_some(self.account_id)
    }

    pub fn linked_services_if_present(&self) -> Option<&str> {
        self.presence
            .linked_services
            .then_some(&self.linked_services)
    }

    pub fn status_message_if_present(&self) -> Option<&str> {
        self.presence.status_message.then_some(&self.status_message)
    }

    pub fn suspended_if_present(&self) -> Option<bool> {
        self.presence.suspended.then_some(self.suspended)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct UserFieldPresence {
    country_iso: bool,
    profile_image_url: bool,
    full_profile_image_url: bool,
    original_profile_image_url: bool,
    account_id: bool,
    linked_services: bool,
    status_message: bool,
    suspended: bool,
}

struct NullableField<T> {
    value: Option<T>,
    present: bool,
}

impl<T> Default for NullableField<T> {
    fn default() -> Self {
        Self {
            value: None,
            present: false,
        }
    }
}

fn deserialize_nullable_field<'de, D, T>(deserializer: D) -> Result<NullableField<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let value = Option::<T>::deserialize(deserializer)?;

    Ok(NullableField {
        present: value.is_some(),
        value,
    })
}

#[derive(Deserialize)]
struct UserWire {
    #[serde(rename = "userId")]
    user_id: i64,

    #[serde(rename = "nickName")]
    nickname: String,

    #[serde(
        default,
        rename = "countryIso",
        deserialize_with = "deserialize_nullable_field"
    )]
    country_iso: NullableField<String>,

    #[serde(
        default,
        rename = "profileImageUrl",
        deserialize_with = "deserialize_nullable_field"
    )]
    profile_image_url: NullableField<String>,

    #[serde(
        default,
        rename = "fullProfileImageUrl",
        deserialize_with = "deserialize_nullable_field"
    )]
    full_profile_image_url: NullableField<String>,

    #[serde(
        default,
        rename = "originalProfileImageUrl",
        deserialize_with = "deserialize_nullable_field"
    )]
    original_profile_image_url: NullableField<String>,

    #[serde(
        default,
        rename = "type",
        deserialize_with = "deserialize_nullable_field"
    )]
    user_type: NullableField<i32>,

    #[serde(
        default,
        rename = "accountId",
        deserialize_with = "deserialize_nullable_field"
    )]
    account_id: NullableField<i64>,

    #[serde(
        default,
        rename = "linkedServices",
        deserialize_with = "deserialize_nullable_field"
    )]
    linked_services: NullableField<String>,

    #[serde(
        default,
        rename = "statusMessage",
        deserialize_with = "deserialize_nullable_field"
    )]
    status_message: NullableField<String>,

    #[serde(default, deserialize_with = "deserialize_nullable_field")]
    suspended: NullableField<bool>,
}

impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = UserWire::deserialize(deserializer)?;
        let presence = UserFieldPresence {
            country_iso: wire.country_iso.present,
            profile_image_url: wire.profile_image_url.present,
            full_profile_image_url: wire.full_profile_image_url.present,
            original_profile_image_url: wire.original_profile_image_url.present,
            account_id: wire.account_id.present,
            linked_services: wire.linked_services.present,
            status_message: wire.status_message.present,
            suspended: wire.suspended.present,
        };

        Ok(Self {
            user_id: wire.user_id,
            nickname: wire.nickname,
            country_iso: wire.country_iso.value.unwrap_or_default(),
            profile_image_url: wire.profile_image_url.value.unwrap_or_default(),
            full_profile_image_url: wire.full_profile_image_url.value.unwrap_or_default(),
            original_profile_image_url: wire.original_profile_image_url.value.unwrap_or_default(),
            user_type: wire.user_type.value.unwrap_or_default(),
            account_id: wire.account_id.value.unwrap_or_default(),
            linked_services: wire.linked_services.value.unwrap_or_default(),
            status_message: wire.status_message.value.unwrap_or_default(),
            suspended: wire.suspended.value.unwrap_or_default(),
            presence,
        })
    }
}
