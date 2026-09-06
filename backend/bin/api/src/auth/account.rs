use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use kiwi_talk_system::get_system_info;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedAccount {
    pub profile: String,

    pub name: String,

    pub email: String,

    #[serde(with = "serde_byte_array")]
    pub token: Option<[u8; 64]>,
}

fn file_path() -> PathBuf {
    get_system_info().config_dir.join("saved_account")
}

pub async fn read() -> anyhow::Result<Option<SavedAccount>> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let path = file_path();
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        anyhow::ensure!(
            metadata.file_type().is_file(),
            "saved account path is not a regular file"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        let reader = BufReader::new(File::open(path)?);

        Ok(bincode::deserialize_from(reader)?)
    })
    .await?
}

pub async fn write(data: Option<SavedAccount>) -> anyhow::Result<()> {
    spawn_blocking(move || -> anyhow::Result<_> {
        let path = file_path();
        let parent = path.parent().context("saved account path has no parent")?;
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
    })
    .await?
    .context("cannot save login data")
}
