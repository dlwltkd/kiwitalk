mod constants;

use std::{
    fmt,
    ops::Deref,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::Context;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use tauri::{
    generate_handler,
    path::PathResolver,
    plugin::{Builder, TauriPlugin},
    Runtime,
};
use tokio::{fs, io::AsyncWriteExt};

use crate::constants::{
    ANDROID_SUBDEVICE_UUID_DOMAIN, APP_DEVICE_UUID_FILE, APP_PORTABLE_DATA_DIR,
    DEFAULT_DEVICE_LOCALE, DEFAULT_DEVICE_NAME,
};

static SYSTEM: OnceLock<SystemInfo> = OnceLock::new();

pub fn get_system_info() -> &'static SystemInfo {
    SYSTEM.get().unwrap()
}

pub async fn init<R: Runtime>(path_resolver: PathResolver<R>) -> anyhow::Result<TauriPlugin<R>> {
    SYSTEM
        .set(create_system_info(&path_resolver).await?)
        .expect("Cannot initialize System information");

    Ok(Builder::new("system")
        .invoke_handler(generate_handler![get_device_locale, get_device_name])
        .build())
}

#[tauri::command]
fn get_device_locale() -> String {
    get_system_info().device.locale.clone()
}

#[tauri::command]
fn get_device_name() -> String {
    get_system_info().device.name.clone()
}

#[derive(Debug)]
/// Various informations for appliaction
pub struct SystemInfo {
    /// Data directory path
    pub data_dir: PathBuf,

    /// Config directory path
    pub config_dir: PathBuf,

    /// Device information
    pub device: Device,
}

#[derive(Debug)]
pub struct Device {
    /// Device locale
    pub locale: String,

    /// Device name
    pub name: String,

    /// Generated unique id
    pub device_uuid: DeviceUuid,
}

impl Device {
    #[inline]
    pub fn language(&self) -> &str {
        locale_language(&self.locale).unwrap_or("en")
    }
}

fn locale_language(locale: &str) -> Option<&str> {
    locale
        .split(['-', '_', '.'])
        .next()
        .filter(|language| language.len() == 2 && language.chars().all(|c| c.is_ascii_alphabetic()))
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeviceUuid(String);

impl DeviceUuid {
    pub fn new(data: &[u8; 64]) -> Self {
        DeviceUuid(STANDARD.encode(data))
    }

    pub fn decode(&self) -> Vec<u8> {
        STANDARD.decode(&self.0).unwrap()
    }

    pub fn android_subdevice_uuid(&self) -> String {
        let bytes: [u8; 64] = self.decode().try_into().unwrap();
        derive_android_subdevice_uuid(&bytes)
    }
}

impl fmt::Debug for DeviceUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceUuid(REDACTED)")
    }
}

impl Deref for DeviceUuid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn gen_device_uuid() -> DeviceUuid {
    let mut rng = rand::thread_rng();

    let mut random_bytes = [0_u8; 64];
    rng.fill(&mut random_bytes);

    DeviceUuid::new(&random_bytes)
}

pub fn derive_android_subdevice_uuid(device_seed: &[u8; 64]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ANDROID_SUBDEVICE_UUID_DOMAIN);
    hasher.update(device_seed);
    hex::encode(hasher.finalize())
}

async fn create_system_info<R: Runtime>(resolver: &PathResolver<R>) -> anyhow::Result<SystemInfo> {
    let device_data_dir = resolver
        .app_data_dir()
        .context("cannot find device data directory")?;

    let device_config_dir = resolver
        .app_config_dir()
        .context("cannot find device config directory")?;

    let device_uuid = init_device_uuid(&device_data_dir)
        .await
        .context("cannot initialize device uuid")?;

    let (data_dir, config_dir) = if fs::metadata(APP_PORTABLE_DATA_DIR)
        .await
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
    {
        let data_dir = PathBuf::from(APP_PORTABLE_DATA_DIR);

        (data_dir.clone(), data_dir)
    } else {
        (device_data_dir, device_config_dir)
    };

    let locale = sys_locale::get_locale()
        .filter(|locale| locale_language(locale).is_some())
        .unwrap_or_else(|| String::from(DEFAULT_DEVICE_LOCALE));
    let name = hostname::get()
        .map(|hostname| hostname.into_string().ok())
        .ok()
        .flatten()
        .unwrap_or_else(|| String::from(DEFAULT_DEVICE_NAME));

    let device_info = Device {
        locale,
        name,
        device_uuid,
    };

    tokio::try_join!(
        fs::create_dir_all(&data_dir),
        fs::create_dir_all(&config_dir)
    )
    .context("failed to create data directories")?;

    Ok(SystemInfo {
        data_dir,
        config_dir,
        device: device_info,
    })
}

async fn init_device_uuid(device_data_dir: &Path) -> anyhow::Result<DeviceUuid> {
    let path = device_data_dir.join(APP_DEVICE_UUID_FILE);

    Ok(
        if fs::metadata(&path)
            .await
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            let data = fs::read(&path).await?;
            let buf: [u8; 64] = data.try_into().map_err(|data: Vec<u8>| {
                anyhow::anyhow!(
                    "device UUID must contain exactly 64 bytes, found {}",
                    data.len()
                )
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await?;
            }

            DeviceUuid::new(&buf)
        } else {
            let uuid = gen_device_uuid();
            fs::create_dir_all(path.parent().unwrap()).await?;
            write_device_uuid(&path, &uuid.decode()).await?;

            uuid
        },
    )
}

#[cfg(unix)]
async fn write_device_uuid(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .await?;
    file.write_all(data).await
}

#[cfg(not(unix))]
async fn write_device_uuid(path: &Path, data: &[u8]) -> std::io::Result<()> {
    fs::write(path, data).await
}

#[cfg(test)]
mod tests {
    use super::{derive_android_subdevice_uuid, locale_language, DeviceUuid};

    #[test]
    fn extracts_two_letter_language_from_common_locale_formats() {
        assert_eq!(locale_language("ko-KR"), Some("ko"));
        assert_eq!(locale_language("en_US.UTF-8"), Some("en"));
    }

    #[test]
    fn rejects_non_language_locales() {
        assert_eq!(locale_language("C"), None);
        assert_eq!(locale_language("POSIX"), None);
        assert_eq!(locale_language(""), None);
    }

    #[test]
    fn derives_a_stable_domain_separated_android_uuid() {
        let uuid = derive_android_subdevice_uuid(&[7_u8; 64]);

        assert_eq!(uuid.len(), 64);
        assert!(uuid.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(uuid, derive_android_subdevice_uuid(&[7_u8; 64]));
        assert_ne!(uuid, hex::encode([7_u8; 32]));
    }

    #[test]
    fn device_uuid_debug_is_redacted() {
        let uuid = DeviceUuid::new(&[7_u8; 64]);

        assert_eq!(format!("{uuid:?}"), "DeviceUuid(REDACTED)");
        assert!(!format!("{uuid:?}").contains(&*uuid));
    }
}
