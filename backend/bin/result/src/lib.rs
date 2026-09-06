use serde::Serialize;
use std::ops::Deref;

pub type TauriResult<T> = Result<T, TauriAnyhowError>;

#[derive(Debug)]
#[repr(transparent)]
pub struct TauriAnyhowError(anyhow::Error);

impl<T: Into<anyhow::Error>> From<T> for TauriAnyhowError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

impl Deref for TauriAnyhowError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for TauriAnyhowError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}
