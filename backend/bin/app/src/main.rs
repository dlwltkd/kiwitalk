#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod configuration;

use log::info;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, DeviceEventFilter, Manager, RunEvent, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

#[cfg(target_os = "windows")]
use window_vibrancy::apply_acrylic;

fn create_main_window<R: Runtime>(manager: &impl Manager<R>) -> anyhow::Result<WebviewWindow<R>> {
    let window_builder =
        WebviewWindowBuilder::new(manager, "main", WebviewUrl::App("index.html".into()))
            .inner_size(1280.0, 720.0)
            .min_inner_size(640.0, 420.0)
            .resizable(true)
            .title("KiwiTalk")
            .decorations(false)
            .visible(false);

    #[cfg(target_os = "linux")]
    let window = window_builder.build()?;
    #[cfg(not(target_os = "linux"))]
    let window = window_builder.transparent(true).build()?;

    #[cfg(target_os = "macos")]
    #[allow(deprecated)]
    apply_vibrancy(
        &window,
        NSVisualEffectMaterial::MediumLight,
        None,
        Some(10.0),
    )
    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

    #[cfg(target_os = "windows")]
    apply_acrylic(&window, Some((0, 0, 0, 125)))
        .expect("Unsupported platform! 'apply_acrylic' is only supported on Windows");

    Ok(window)
}

async fn init_app_plugins(handle: &AppHandle<impl Runtime>) -> anyhow::Result<()> {
    handle.plugin(kiwi_talk_system::init(handle.path().clone()).await?)?;
    handle.plugin(kiwi_talk_api::init().await?)?;
    handle.plugin(configuration::init_plugin("configuration").await?)?;
    handle.plugin(kiwi_talk_client::init("client").await?)?;

    Ok(())
}

fn show_window(app: &AppHandle<impl Runtime>) {
    let main_window = if let Some(window) = app.get_webview_window("main") {
        window
    } else {
        let window = create_main_window(app).unwrap();
        window.show().unwrap();

        window
    };

    main_window.unminimize().ok();
    main_window.show().ok();
    main_window.set_focus().ok();
}

fn create_tray<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit KiwiTalk", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let log_plugin = tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::LogDir { file_name: None }),
            Target::new(TargetKind::Stderr),
            Target::new(TargetKind::Webview),
        ])
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        })
        .build();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            info!(
                "app: {} argv: {argv:?} cwd: {cwd:?}",
                app.package_info().name
            );
            show_window(app);
        }))
        .plugin(log_plugin)
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .device_event_filter(DeviceEventFilter::Never)
        .build(tauri::generate_context!())?;

    if let Err(err) = init_app_plugins(app.handle()).await {
        app.dialog()
            .message(format!("{err:?}"))
            .title("KiwiTalk Startup Fatal Error")
            .blocking_show();
        app.run(|_, _| {});

        return Ok(());
    }

    create_tray(app.handle())?;

    let main_window = create_main_window(&app)?;
    main_window.show()?;

    #[cfg(debug_assertions)]
    {
        main_window.open_devtools();
    }

    app.run(|_, event| {
        if let RunEvent::ExitRequested { code, api, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });

    Ok(())
}
