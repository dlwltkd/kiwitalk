fn main() {
    use tauri_build::{Attributes, DefaultPermissionRule, InlinedPlugin};

    fn plugin(commands: &'static [&'static str]) -> InlinedPlugin {
        InlinedPlugin::new()
            .commands(commands)
            .default_permission(DefaultPermissionRule::AllowAllCommands)
    }

    tauri_build::try_build(Attributes::new().plugins([
        (
            "api",
            plugin(&[
                "logon",
                "login",
                "logout",
                "auto_login",
                "poll_android_registration",
                "cancel_android_registration",
                "default_login_form",
                "me_profile",
                "friend_profile",
                "update_friends",
            ]),
        ),
        (
            "client",
            plugin(&[
                "created",
                "create",
                "destroy",
                "next_event",
                "channel_list",
                "load_channel",
                "channel_set_active",
                "channel_sync_history",
                "channel_send_text",
                "channel_load_chat",
                "channel_load_archive",
                "channel_import_archive",
                "normal_channel_read_chat",
            ]),
        ),
        ("configuration", plugin(&["load", "save"])),
        ("system", plugin(&["get_device_locale", "get_device_name"])),
    ]))
    .expect("failed to build KiwiTalk's Tauri configuration");
}
