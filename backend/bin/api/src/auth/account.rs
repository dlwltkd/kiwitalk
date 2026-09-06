use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::task::spawn_blocking;
use zeroize::Zeroize;

use kiwi_talk_system::get_system_info;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedAccount {
    pub profile: String,

    pub name: String,

    pub email: String,

    #[serde(with = "serde_byte_array")]
    pub token: Option<[u8; 64]>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedSession {
    pub email: String,
    pub user_id: u64,
    pub device_uuid: String,
    pub refresh_token: String,
}

impl Drop for SavedSession {
    fn drop(&mut self) {
        self.refresh_token.zeroize();
    }
}

fn file_path() -> PathBuf {
    get_system_info().config_dir.join("saved_account")
}

fn session_file_path() -> PathBuf {
    get_system_info().config_dir.join("saved_session")
}

fn read_path<T: DeserializeOwned>(path: PathBuf) -> anyhow::Result<Option<T>> {
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "saved credential path is not a regular file"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(bincode::deserialize_from(BufReader::new(File::open(
        path,
    )?))?)
}

fn write_path<T: Serialize>(path: PathBuf, data: Option<T>) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("saved credential path has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let file = options.open(&temporary)?;
    let mut writer = BufWriter::new(file);

    let result = (|| -> anyhow::Result<()> {
        bincode::serialize_into(&mut writer, &data)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        std::fs::rename(&temporary, &path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }

    result
}

pub async fn read() -> anyhow::Result<Option<SavedAccount>> {
    spawn_blocking(move || read_path(file_path())).await?
}

pub async fn write(data: Option<SavedAccount>) -> anyhow::Result<()> {
    spawn_blocking(move || write_path(file_path(), data))
        .await?
        .context("cannot save login data")
}

pub async fn read_session() -> anyhow::Result<Option<SavedSession>> {
    spawn_blocking(move || read_path(session_file_path())).await?
}

pub async fn write_session(data: Option<SavedSession>) -> anyhow::Result<()> {
    spawn_blocking(move || write_path(session_file_path(), data))
        .await?
        .context("cannot save login session")
}
