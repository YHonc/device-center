//! Native lifetime polling: independent of the main WebView and frontend timers.
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

const FOREGROUND: u64 = 30;
const BACKGROUND: u64 = 120;
const MAX_BACKOFF: u64 = 900;
const LOW: u64 = 20;
// Hysteresis avoids a new episode when a rounded reading oscillates at 20%.
const RECOVERED: u64 = 25;
const FRESH_MS: u64 = 300_000;

#[derive(Default)]
struct Policy {
    failures: u32,
    alerted: bool,
}
impl Policy {
    fn delay(&self, foreground: bool) -> Duration {
        let base = if foreground { FOREGROUND } else { BACKGROUND };
        Duration::from_secs(
            base.saturating_mul(1u64 << self.failures.min(5))
                .min(MAX_BACKOFF),
        )
    }
    fn failed(&mut self) {
        self.failures = self.failures.saturating_add(1);
    }
    fn observe(&mut self, sample: &Value, now_ms: u64) -> bool {
        self.failures = 0;
        let Some(sampled_at) = sample["sampledAt"].as_u64() else {
            return false;
        };
        if sample["stale"].as_bool() == Some(true)
            || sampled_at > now_ms
            || now_ms.saturating_sub(sampled_at) > FRESH_MS
        {
            return false;
        }
        let Some(percent) = sample["batteryPercent"].as_u64().filter(|p| *p <= 100) else {
            return false;
        };
        if sample["charging"].as_bool() == Some(true) || percent >= RECOVERED {
            self.alerted = false;
            return false;
        }
        if percent <= LOW && !self.alerted {
            self.alerted = true;
            return true;
        }
        false
    }
}
struct Device {
    policy: Policy,
    next: Instant,
    connected: bool,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Call once from setup. The worker owns no WebView and continues after main closes.
pub(crate) fn start(app: tauri::AppHandle) {
    // A UDP timeout or the lamp's serialized command queue must never delay HID
    // reads and low-battery notifications. Both workers live without a WebView.
    start_lamp(app.clone());
    std::thread::spawn(move || {
        let mut devices: HashMap<String, Device> = HashMap::new();
        let mut next_scan = Instant::now();
        let mut scan_policy = Policy::default();
        let mut was_foreground = false;
        loop {
            let foreground = app
                .get_webview_window("main")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            let now = Instant::now();
            // Reopening the main window refreshes healthy devices promptly. Errors retain backoff.
            if foreground && !was_foreground {
                next_scan = now;
                for device in devices.values_mut().filter(|d| d.policy.failures == 0) {
                    device.next = now;
                }
            }
            was_foreground = foreground;
            if now >= next_scan {
                match hidapi::HidApi::new() {
                    Ok(api) => {
                        scan_policy.failures = 0;
                        let found: HashSet<String> = api
                            .device_list()
                            .filter(|d| crate::battery_match(d))
                            .map(|d| hex::encode(d.path().to_bytes()))
                            .collect();
                        for id in &found {
                            let device = devices.entry(id.clone()).or_insert_with(|| Device {
                                policy: Policy::default(),
                                next: now,
                                connected: true,
                            });
                            if !device.connected {
                                device.next = now;
                            }
                            device.connected = true;
                        }
                        for (id, device) in devices.iter_mut() {
                            if device.connected && !found.contains(id) {
                                device.connected = false;
                                // Preserve the episode across unplug/sleep/reconnect.
                                let sample = crate::cache_device(
                                    &app,
                                    id,
                                    json!({"deviceId":id,"stale":true,"connection":"disconnected","error":"设备已断开"}),
                                );
                                let _ = app.emit("battery-update", sample);
                            }
                        }
                    }
                    Err(error) => {
                        scan_policy.failed();
                        // Enumeration failure is not evidence of unplugging or recovery.
                        for id in devices.keys() {
                            let sample = crate::cache_device(
                                &app,
                                id,
                                json!({"deviceId":id,"stale":true,"connection":"unknown","error":error.to_string()}),
                            );
                            let _ = app.emit("battery-update", sample);
                        }
                    }
                }
                next_scan = Instant::now() + scan_policy.delay(foreground);
            }
            for (id, device) in devices
                .iter_mut()
                .filter(|(_, d)| d.connected && now >= d.next)
            {
                match crate::query_battery_sync(id) {
                    Ok(mut sample) => {
                        let notify = device.policy.observe(&sample, now_ms());
                        if let Some(object) = sample.as_object_mut() {
                            object.insert("deviceId".into(), json!(id));
                            object.insert("stale".into(), json!(false));
                            object.insert("connection".into(), json!("connected"));
                        }
                        if notify {
                            let model = sample["model"].as_str().unwrap_or("设备");
                            let _ = app
                                .notification()
                                .builder()
                                .title("设备电量偏低")
                                .body(format!(
                                    "{} 剩余 {}%，请及时充电。",
                                    model, sample["batteryPercent"]
                                ))
                                .show();
                        }
                        let sample = crate::cache_device(&app, id, sample);
                        let _ = app.emit("battery-update", sample);
                    }
                    Err(error) => {
                        device.policy.failed();
                        let sample = crate::cache_device(
                            &app,
                            id,
                            json!({"deviceId":id,"error":error,"stale":true,"connection":"unknown"}),
                        );
                        let _ = app.emit("battery-update", sample);
                    }
                }
                device.next = Instant::now() + device.policy.delay(foreground);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[derive(Default)]
struct LampSchedule {
    policy: Policy,
    configured: bool,
    foreground: bool,
    next: Option<Instant>,
    last_success: Option<Instant>,
    stale: bool,
}

impl LampSchedule {
    fn due(&mut self, configured: bool, foreground: bool, now: Instant) -> bool {
        if !configured {
            *self = Self::default();
            return false;
        }
        if !self.configured || (foreground && !self.foreground && self.policy.failures == 0) {
            self.next = Some(now);
        }
        self.configured = true;
        self.foreground = foreground;
        self.next.is_none_or(|next| now >= next)
    }

    fn completed(&mut self, success: bool, now: Instant) {
        if success {
            self.policy.failures = 0;
            self.last_success = Some(now);
            self.stale = false;
        } else {
            self.policy.failed();
            self.stale = true;
        }
        self.next = Some(now + self.policy.delay(self.foreground));
    }

    fn expired(&mut self, now: Instant) -> bool {
        if !self.stale
            && self.last_success.is_some_and(|last| {
                now.saturating_duration_since(last) > Duration::from_millis(FRESH_MS)
            })
        {
            self.stale = true;
            return true;
        }
        false
    }
}

fn start_lamp(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut schedule = LampSchedule::default();
        loop {
            let foreground = app
                .get_webview_window("main")
                .and_then(|w| w.is_visible().ok())
                .unwrap_or(false);
            let due = schedule.due(
                crate::lamp_monitor_configured(&app),
                foreground,
                Instant::now(),
            );
            if schedule.expired(Instant::now()) {
                let sample = crate::cache_lamp(
                    &app,
                    &json!({"stale":true,"connection":"unknown","error":"挂灯状态已过期，请重新读取"}),
                );
                let _ = app.emit("lamp-update", sample);
            }
            if due {
                let result = crate::poll_lamp(&app);
                schedule.completed(result.is_ok(), Instant::now());
                let sample = match result {
                    Ok(sample) => sample,
                    Err(error) => crate::cache_lamp(
                        &app,
                        &json!({"stale":true,"connection":"unknown","error":error}),
                    ),
                };
                let _ = app.emit("lamp-update", sample);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lamp_mock_schedule_skips_unconfigured_and_reopens_healthy() {
        let start = Instant::now();
        let mut schedule = LampSchedule::default();
        // The mock invokes no transport unless the native scheduler says due.
        let mut reads = 0;
        for second in 0..300 {
            if schedule.due(false, true, start + Duration::from_secs(second)) {
                reads += 1;
            }
        }
        assert_eq!(reads, 0);
        assert!(schedule.due(true, false, start));
        schedule.completed(true, start);
        assert!(!schedule.due(true, false, start + Duration::from_secs(119)));
        assert!(schedule.due(true, true, start + Duration::from_secs(119)));
        schedule.completed(true, start + Duration::from_secs(119));
        assert!(!schedule.due(true, true, start + Duration::from_secs(148)));
        assert!(schedule.due(true, true, start + Duration::from_secs(149)));
    }

    #[test]
    fn lamp_mock_failure_backoff_preserves_stale_until_success() {
        let start = Instant::now();
        let mut schedule = LampSchedule::default();
        assert!(schedule.due(true, false, start));
        schedule.completed(true, start);
        assert!(schedule.expired(start + Duration::from_secs(301)));
        assert!(!schedule.expired(start + Duration::from_secs(302)));
        schedule.completed(false, start);
        assert!(!schedule.due(true, true, start + Duration::from_secs(239)));
        assert!(schedule.due(true, true, start + Duration::from_secs(240)));
        for _ in 0..100 {
            schedule.completed(false, start);
        }
        assert_eq!(schedule.next.unwrap().duration_since(start).as_secs(), 900);
        schedule.completed(true, start);
        assert!(!schedule.stale);
        assert_eq!(schedule.next.unwrap().duration_since(start).as_secs(), 30);
    }
    fn sample(percent: Value, charging: Value) -> Value {
        json!({"batteryPercent":percent,"charging":charging,"sampledAt":1000})
    }
    #[test]
    fn intervals_backoff_and_success() {
        let mut p = Policy::default();
        assert_eq!(p.delay(true).as_secs(), 30);
        assert_eq!(p.delay(false).as_secs(), 120);
        p.failed();
        assert_eq!(p.delay(true).as_secs(), 60);
        for _ in 0..100 {
            p.failed();
        }
        assert_eq!(p.delay(false).as_secs(), 900);
        p.observe(&sample(json!(80), Value::Null), 1000);
        assert_eq!(p.delay(false).as_secs(), 120);
    }
    #[test]
    fn episode_survives_errors_and_resets_only_on_recovery_or_charge() {
        let mut p = Policy::default();
        assert!(p.observe(&sample(json!(20), Value::Null), 1000));
        p.failed();
        assert!(!p.observe(&sample(json!(19), json!(false)), 1000));
        assert!(!p.observe(&sample(json!(21), json!(false)), 1000));
        assert!(!p.observe(&sample(json!(20), json!(false)), 1000));
        assert!(!p.observe(&sample(json!(25), json!(false)), 1000));
        assert!(p.observe(&sample(json!(20), json!(false)), 1000));
        assert!(!p.observe(&sample(json!(10), json!(true)), 1000));
        assert!(p.observe(&sample(json!(10), json!(false)), 1000));
    }
    #[test]
    fn unknown_stale_invalid_or_future_readings_cannot_alert_or_reset() {
        let mut p = Policy::default();
        for value in [
            sample(Value::Null, Value::Null),
            sample(json!(101), json!(false)),
            json!({"batteryPercent":5}),
            json!({"batteryPercent":5,"sampledAt":1001}),
            json!({"batteryPercent":5,"sampledAt":1000,"stale":true}),
        ] {
            assert!(!p.observe(&value, 1000));
        }
        assert!(!p.observe(&sample(json!(5), json!(false)), 301_001));
        assert!(p.observe(&sample(json!(5), json!(false)), 1000));
        assert!(!p.observe(&sample(json!(90), json!(true)), 301_001));
        assert!(!p.observe(&sample(json!(5), json!(false)), 1000));
    }
}
