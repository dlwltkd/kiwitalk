use talk_api_internal::{
    auth::android::{AndroidSubdeviceProfile, ANDROID_SUBDEVICE_PROFILE},
    config::Config,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolProfile {
    AndroidSubdevice,
}

pub const ACTIVE_PROTOCOL_PROFILE: ProtocolProfile = ProtocolProfile::AndroidSubdevice;

impl ProtocolProfile {
    pub const fn api_profile(self) -> AndroidSubdeviceProfile {
        match self {
            Self::AndroidSubdevice => ANDROID_SUBDEVICE_PROFILE,
        }
    }

    pub const fn api_config(self) -> Config<'static> {
        self.api_profile().config()
    }

    pub const fn os(self) -> &'static str {
        match self {
            Self::AndroidSubdevice => "android",
        }
    }

    pub const fn app_version(self) -> &'static str {
        self.api_profile().app_version
    }

    pub const fn model(self) -> &'static str {
        self.api_profile().model
    }

    pub const fn language(self) -> &'static str {
        self.api_profile().language
    }

    pub const fn mccmnc(self) -> &'static str {
        "999"
    }

    pub const fn country_iso(self) -> &'static str {
        "KR"
    }

    pub const fn protocol_version(self) -> &'static str {
        "1"
    }

    pub const fn device_type(self) -> Option<i8> {
        None
    }

    pub const fn revision(self) -> Option<i32> {
        Some(0)
    }

    pub const fn include_pc_status(self) -> bool {
        false
    }

    pub const fn background(self) -> Option<bool> {
        Some(false)
    }

    pub const fn last_chat_id(self) -> Option<i64> {
        None
    }

    pub const fn is_switching(self) -> Option<bool> {
        Some(false)
    }

    pub const fn use_sub(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_profile_is_android_end_to_end() {
        let profile = ACTIVE_PROTOCOL_PROFILE;

        assert_eq!(profile.os(), "android");
        assert_eq!(profile.app_version(), "26.7.2");
        assert_eq!(profile.model(), "SM-X930");
        assert_eq!(profile.api_config().agent.agent(), profile.os());
        assert_eq!(profile.protocol_version(), "1");
        assert_eq!(profile.device_type(), None);
        assert_eq!(profile.revision(), Some(0));
        assert!(!profile.include_pc_status());
        assert_eq!(profile.background(), Some(false));
        assert_eq!(profile.last_chat_id(), None);
        assert_eq!(profile.is_switching(), Some(false));
    }
}
