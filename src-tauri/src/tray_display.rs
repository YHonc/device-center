//! Native tray preferences and snapshot-only rendering. Never performs device IO.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{tray::TrayIconBuilder, Manager};
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    #[default]
    Logo,
    Mouse,
    Keyboard,
    Both,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Preferences {
    mode: Mode,
}
pub(crate) struct Store {
    file: PathBuf,
    preferences: Mutex<Preferences>,
    drawn: Mutex<Vec<String>>,
}
#[tauri::command]
pub(crate) fn get_tray_preferences(app: tauri::AppHandle) -> Preferences {
    app.state::<Store>()
        .preferences
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
#[tauri::command]
pub(crate) fn set_tray_preferences(
    preferences: Preferences,
    app: tauri::AppHandle,
) -> Result<Preferences, String> {
    let store = app.state::<Store>();
    std::fs::create_dir_all(store.file.parent().ok_or("配置目录无效")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(
        &store.file,
        serde_json::to_vec(&preferences).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    *store.preferences.lock().map_err(|_| "托盘配置锁异常")? = preferences.clone();
    refresh(&app);
    Ok(preferences)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn local_time(stamp: u64) -> String {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct LocalTime {
            year: u16,
            month: u16,
            weekday: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            millisecond: u16,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn FileTimeToLocalFileTime(input: *const u64, output: *mut u64) -> i32;
            fn FileTimeToSystemTime(input: *const u64, output: *mut LocalTime) -> i32;
        }
        if let Some(ticks) = stamp
            .checked_mul(10000)
            .and_then(|v| v.checked_add(116444736000000000))
        {
            let mut local = 0;
            let mut time = LocalTime {
                year: 0,
                month: 0,
                weekday: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
                millisecond: 0,
            };
            // Windows converts the timestamp with the current user's local timezone.
            if unsafe {
                FileTimeToLocalFileTime(&ticks, &mut local) != 0
                    && FileTimeToSystemTime(&local, &mut time) != 0
            } {
                return format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    time.year, time.month, time.day, time.hour, time.minute, time.second
                );
            }
        }
    }
    format!("Unix ms {stamp}")
}
fn reading(sample: Option<&Value>, time: u64) -> (String, String) {
    let Some(v) = sample else {
        return ("?".into(), "未知：尚无读数".into());
    };
    let stamp = v["sampledAt"].as_u64().unwrap_or(0);
    let stale =
        v["stale"] == true || stamp == 0 || stamp > time || time.saturating_sub(stamp) > 300000;
    let pct = v["batteryPercent"].as_u64().filter(|v| *v <= 100);
    let text = if stale {
        "!".into()
    } else {
        pct.map(|p| p.to_string()).unwrap_or("?".into())
    };
    (
        text,
        format!(
            "采样时间：{}；{}",
            local_time(stamp),
            if stale {
                "读数过期"
            } else if pct.is_none() {
                "电量未知"
            } else {
                "有效读数"
            }
        ),
    )
}
const DIGITS: [[u8; 5]; 12] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
    [7, 1, 2, 0, 2],
    [2, 2, 2, 0, 2],
];
fn render(text: &str, size: u32, keyboard: bool) -> Vec<u8> {
    let mut pixels = vec![0; (size * size * 4) as usize];
    let scale = (size / (if text.len() > 2 { 12 } else { 8 })).max(1);
    let width = (text.len() as u32 * 4 - 1) * scale;
    let x0 = (size - width) / 2;
    let y0 = (size - 5 * scale) / 2;
    for y in 0..size {
        for x in 0..size {
            let p = ((y * size + x) * 4) as usize;
            pixels[p..p + 4].copy_from_slice(if keyboard {
                &[28, 65, 115, 255]
            } else {
                &[24, 82, 62, 255]
            });
        }
    }
    for (n, c) in text.chars().enumerate() {
        let i = match c {
            '0'..='9' => c as usize - '0' as usize,
            '!' => 11,
            _ => 10,
        };
        for y in 0..5 {
            for x in 0..3 {
                if DIGITS[i][y] & (1 << (2 - x)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let p = (((y0 + y as u32 * scale + dy) * size
                                + x0
                                + (n as u32 * 4 + x) * scale
                                + dx)
                                * 4) as usize;
                            pixels[p..p + 4].copy_from_slice(&[255, 255, 255, 255]);
                        }
                    }
                }
            }
        }
    }
    pixels
}
pub(crate) fn refresh(app: &tauri::AppHandle) {
    let Some(store) = app.try_state::<Store>() else {
        return;
    };
    let mut drawn = store.drawn.lock().unwrap_or_else(|e| e.into_inner());
    let mode = store
        .preferences
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .mode;
    let snapshots = app
        .state::<crate::SnapshotStore>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut keys = vec![format!("{mode:?}")];
    for (id, keyboard) in [("battery-mouse", false), ("battery-keyboard", true)] {
        let enabled = matches!(
            (mode, keyboard),
            (Mode::Both, _) | (Mode::Mouse, false) | (Mode::Keyboard, true)
        );
        let sample = snapshots.devices.iter().find(|v| {
            v["model"].as_str().is_some_and(|m| {
                if keyboard {
                    m.to_lowercase().contains("kx75")
                } else {
                    m.to_lowercase().contains("atk")
                }
            })
        });
        let (text, detail) = reading(sample, now());
        let label = if keyboard {
            "迈从 KX75"
        } else {
            "ATK 鼠标"
        };
        let tooltip = format!(
            "{label}：{text}{}\n{detail}",
            if text.chars().all(|c| c.is_ascii_digit()) {
                "%"
            } else {
                ""
            }
        );
        keys.push(format!("{enabled}:{text}"));
        let icon_index = keys.len() - 1;
        keys.push(format!("{enabled}:{tooltip}"));
        if let Some(tray) = app.tray_by_id(id) {
            if drawn.get(icon_index) != keys.get(icon_index) {
                let _ = tray.set_visible(enabled);
                if enabled {
                    let _ = tray.set_icon(Some(tauri::image::Image::new_owned(
                        render(&text, 32, keyboard),
                        32,
                        32,
                    )));
                }
            }
            if enabled && drawn.get(icon_index + 1) != keys.get(icon_index + 1) {
                let _ = tray.set_tooltip(Some(tooltip));
            }
        }
    }
    if drawn.first() != keys.first() {
        if let Some(logo) = app.tray_by_id("logo") {
            let _ = logo.set_visible(mode == Mode::Logo);
        }
    }
    *drawn = keys;
}
fn load_preferences(file: &std::path::Path) -> Preferences {
    std::fs::read(file)
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_default()
}
pub(crate) fn start(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let file = app.path().app_config_dir()?.join("tray-preferences.json");
    let preferences = load_preferences(&file);
    app.manage(Store {
        file,
        preferences: Mutex::new(preferences),
        drawn: Mutex::new(vec![]),
    });
    for id in ["battery-mouse", "battery-keyboard"] {
        use tauri::menu::{Menu, MenuItem};
        let show_id = format!("{id}-show");
        let quit_id = format!("{id}-quit");
        let show = MenuItem::with_id(app, show_id.clone(), "打开设备中心", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, quit_id.clone(), "退出", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&show, &quit])?;
        TrayIconBuilder::with_id(id)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(move |app, event| {
                if event.id.as_ref() == show_id {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let _ = crate::show_main_window_sync(app);
                    });
                } else if event.id.as_ref() == quit_id {
                    crate::request_tray_exit(app);
                }
            })
            .icon(tauri::image::Image::new_owned(
                render("?", 32, id.ends_with("keyboard")),
                32,
                32,
            ))
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        rect,
                        ..
                    } = event {
                    let app = tray.app_handle().clone();
                    std::thread::spawn(move || {
                        let _ = crate::show_compact_window(&app, Some(rect));
                    });
                }
            })
            .build(app)?;
    }
    refresh(app);
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(15));
        refresh(&app);
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sizes_and_symbols() {
        for size in [16, 20, 24, 32] {
            for text in ["0", "55", "100", "?", "!"] {
                let image = render(text, size, false);
                assert_eq!(image.len(), (size * size * 4) as usize);
                assert!(image.chunks(4).any(|p| p == [255, 255, 255, 255]));
                if let Ok(folder) = std::env::var("DEVICE_CENTER_TRAY_PREVIEW_DIR") {
                    std::fs::create_dir_all(&folder).unwrap();
                    let suffix = match text {
                        "?" => "unknown",
                        "!" => "stale",
                        v => v,
                    };
                    std::fs::write(
                        std::path::Path::new(&folder).join(format!("battery-{size}-{suffix}.rgba")),
                        &image,
                    )
                    .unwrap();
                }
            }
        }
    }
    #[test]
    fn freshness_explicit() {
        assert_eq!(reading(None, 400000).0, "?");
        assert_eq!(
            reading(
                Some(&serde_json::json!({"sampledAt":1,"batteryPercent":55})),
                400000
            )
            .0,
            "!"
        );
        assert_eq!(
            reading(
                Some(&serde_json::json!({"sampledAt":399999,"batteryPercent":55})),
                400000
            )
            .0,
            "55"
        );
    }
    #[test]
    fn modes_roundtrip() {
        for mode in [Mode::Logo, Mode::Mouse, Mode::Keyboard, Mode::Both] {
            let p = Preferences { mode };
            assert_eq!(
                serde_json::from_slice::<Preferences>(&serde_json::to_vec(&p).unwrap()).unwrap(),
                p
            );
        }
        assert!(serde_json::from_str::<Preferences>("{\"mode\":\"unknown\"}").is_err());
    }
    #[test]
    fn saved_preferences_restore_and_corruption_defaults() {
        let path = std::env::temp_dir().join(format!(
            "device-center-tray-mock-{}.json",
            rand::random::<u64>()
        ));
        let expected = Preferences { mode: Mode::Both };
        std::fs::write(&path, serde_json::to_vec(&expected).unwrap()).unwrap();
        assert_eq!(load_preferences(&path), expected);
        std::fs::write(&path, b"invalid").unwrap();
        assert_eq!(load_preferences(&path), Preferences::default());
        std::fs::remove_file(path).unwrap();
        assert!(local_time(1700000000000).contains("2023"));
    }
}
