//! Explicit opt-in local acceptance harness. Never enabled in shipped builds.
use super::{cache_device, cloud, settings, show_compact_window, show_main_window_sync, HidDevice};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
static PAGE: AtomicUsize = AtomicUsize::new(0);
pub(crate) fn root() -> PathBuf {
    PathBuf::from(
        std::env::var_os("DEVICE_CENTER_LIFECYCLE_ROOT")
            .expect("isolated validation root required"),
    )
}
pub(crate) fn isolated_context(
    mut context: tauri::Context<tauri::Wry>,
) -> tauri::Context<tauri::Wry> {
    assert!(root().is_absolute());
    std::fs::create_dir_all(root()).expect("validation directory");
    context.config_mut().app.windows[0].data_directory = Some(root().join("webview-profile"));
    context.config_mut().app.windows[0].title = "LIFECYCLE_VALIDATION_ONLY".into();
    context
}
pub(crate) fn record(event: &str) {
    let line = json!({"event":event,"at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()});
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root().join("events.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}
#[tauri::command]
pub(crate) fn lifecycle_test_log(message: String) -> std::result::Result<(), String> {
    if message.len() > 128
        || !message
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err("invalid marker".into());
    }
    record(&message);
    Ok(())
}
#[tauri::command]
pub(crate) fn get_lamp_config() -> Value {
    json!({"configured":false})
}
#[tauri::command]
pub(crate) fn get_autostart() -> bool {
    false
}
#[tauri::command]
pub(crate) fn list_hid_devices() -> Vec<HidDevice> {
    vec![HidDevice {
        id: "validation-device".into(),
        vendor_id: 0x373b,
        product_id: 0x101a,
        usage_page: 0xff02,
        usage: 2,
        interface_number: 1,
        manufacturer: None,
        product: Some("LIFECYCLE_VALIDATION_ONLY".into()),
        battery_supported: true,
    }]
}
#[tauri::command]
pub(crate) fn get_device_settings(device_id: String) -> settings::SettingsSnapshot {
    settings::lifecycle_fixture(&device_id)
}
pub(crate) fn page(webview: &tauri::Webview, payload: &tauri::webview::PageLoadPayload<'_>) {
    if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
        return;
    }
    let phase = if webview.label() == "main" {
        PAGE.fetch_add(1, Ordering::SeqCst)
    } else {
        99
    };
    let script = include_str!("lifecycle_validation.js").replace("__PHASE__", &phase.to_string());
    let _ = webview.eval(&script);
}
fn wait(app: &tauri::AppHandle, absent: &str) -> bool {
    for _ in 0..150 {
        if app.get_webview_window(absent).is_none() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}
pub(crate) fn start(app: tauri::AppHandle) {
    cache_device(
        &app,
        "validation-device",
        json!({"model":"ATK validation fixture","batteryPercent":55,"charging":false,"stale":false,"connection":"connected","sampledAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64}),
    );
    record("started_no_hardware_no_cloud_network");
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        if !wait(&app, "main") {
            record("FAIL_initial_close_timeout");
            app.exit(2);
            return;
        }
        if !cloud::lifecycle_mock::cancelled() {
            record("FAIL_cloud_not_cancelled");
            app.exit(2);
            return;
        }
        record("cloud_cancelled_and_main_destroyed");
        std::thread::sleep(Duration::from_secs(20));
        record(if app.webview_windows().is_empty() {
            "zero_windows_alive_20s"
        } else {
            "FAIL_zero_windows"
        });
        if show_main_window_sync(app.clone()).is_err() {
            record("FAIL_reopen");
            app.exit(2);
            return;
        }
        std::thread::sleep(Duration::from_secs(4));
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.destroy();
        }
        std::thread::sleep(Duration::from_secs(1));
        if show_compact_window(&app, None).is_err() {
            record("FAIL_compact");
            app.exit(2);
            return;
        }
        if let Some(window) = app.get_webview_window("compact") {
            let size = window
                .inner_size()
                .unwrap()
                .to_logical::<f64>(window.scale_factor().unwrap());
            record(if (size.width - super::compact_position::WIDTH).abs() < 1. && (size.height - super::compact_position::HEIGHT).abs() < 1. {
                "compact_native_size_320_320"
            } else {
                "FAIL_compact_size"
            });
        }
        for _ in 0..50 {
            if std::fs::read_to_string(root().join("events.jsonl"))
                .unwrap_or_default()
                .contains("compact_cache_restored")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if let Some(window) = app.get_webview_window("compact") {
            record("compact_native_close_requested");
            let _ = window.close();
            if wait(&app, "compact") {
                record("compact_native_close_completed");
            } else {
                record("FAIL_compact_native_close");
            }
        } else {
            record("FAIL_compact_closed_before_request");
        }
        std::thread::sleep(Duration::from_secs(20));
        record(if app.webview_windows().is_empty() {
            "all_windows_destroyed_alive_20s"
        } else {
            "FAIL_zero_windows_second"
        });
        let _ = show_main_window_sync(app.clone());
        std::thread::sleep(Duration::from_secs(4));
        record("complete");
        app.exit(0);
    });
}
