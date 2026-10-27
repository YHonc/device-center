mod cloud;
mod compact_position;
mod config;
mod crypto;
#[cfg(feature = "lifecycle-validation")]
mod lifecycle_validation;
mod monitor;
mod protocol;
mod settings;
mod tray_display;

use protocol::Rpc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, State};
use zeroize::Zeroize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("网络错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("设备连接失败: {0}")]
    Connection(std::io::Error),
    #[error("设备拒绝指令，代码 {0}")]
    Device(i64),
    #[error("无效响应: {0}")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn message(text: &str) -> Error {
    Error::Message(text.into())
}
struct Backend {
    file: PathBuf,
    lamp_queue: Mutex<()>,
}
#[derive(Default, Serialize, Clone)]
struct Snapshots {
    devices: Vec<Value>,
    lamp: Option<Value>,
}
#[derive(Default)]
struct SnapshotStore(Mutex<Snapshots>);
fn merge_snapshot(previous: &mut Value, incoming: &Value) {
    if !previous.is_object() {
        *previous = json!({});
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let updated = stamp.max(
        previous["updatedAt"]
            .as_u64()
            .unwrap_or(0)
            .saturating_add(1),
    );
    if let (Some(old), Some(new)) = (previous.as_object_mut(), incoming.as_object()) {
        if new.get("stale") == Some(&json!(false)) {
            old.remove("error");
        }
        for (k, v) in new {
            old.insert(k.clone(), v.clone());
        }
        old.insert("updatedAt".into(), json!(updated));
    }
}
pub(crate) fn cache_device(app: &tauri::AppHandle, id: &str, sample: Value) -> Value {
    let store = app.state::<SnapshotStore>();
    let mut snapshots = store.0.lock().unwrap_or_else(|e| e.into_inner());
    let index = snapshots
        .devices
        .iter()
        .position(|v| v["deviceId"] == id)
        .unwrap_or_else(|| {
            snapshots
                .devices
                .push(json!({"deviceId":id,"transport":"hid","capabilities":["battery"]}));
            snapshots.devices.len() - 1
        });
    let row = &mut snapshots.devices[index];
    merge_snapshot(row, &sample);
    if sample.get("batteryPercent").is_some() && sample["stale"] != true {
        row["lastSeen"] = sample["sampledAt"].clone();
    }
    row.clone()
}
pub(crate) fn cache_lamp(app: &tauri::AppHandle, sample: &Value) -> Value {
    let store = app.state::<SnapshotStore>();
    let mut snapshots = store.0.lock().unwrap_or_else(|e| e.into_inner());
    let row = snapshots.lamp.get_or_insert(
        json!({"transport":"miio","capabilities":["power","brightness","temperature"]}),
    );
    merge_snapshot(row, sample);
    row.clone()
}
#[tauri::command]
fn get_device_snapshots(app: tauri::AppHandle) -> Snapshots {
    let store = app.state::<SnapshotStore>();
    let mut result = store.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    for row in result.devices.iter_mut().chain(result.lamp.iter_mut()) {
        if row["sampledAt"]
            .as_u64()
            .is_some_and(|t| now.saturating_sub(t) > 300_000)
        {
            row["stale"] = json!(true);
        }
    }
    result
}
pub(crate) fn lamp_monitor_configured(app: &tauri::AppHandle) -> bool {
    app.state::<Backend>().file.exists()
}
pub(crate) fn poll_lamp(app: &tauri::AppHandle) -> std::result::Result<Value, String> {
    if !lamp_monitor_configured(app) {
        return Err("挂灯尚未配置".into());
    }
    let result = lamp_operation(&app.state::<Backend>(), None).map_err(|e| e.to_string());
    match result {
        Ok(state) => {
            let mut value = serde_json::to_value(state).map_err(|e| e.to_string())?;
            value["stale"] = json!(false);
            value["connection"] = json!("connected");
            value["lastSeen"] = value["sampledAt"].clone();
            Ok(cache_lamp(app, &value))
        }
        Err(e) => {
            cache_lamp(app, &json!({"stale":true,"connection":"unknown","error":e}));
            Err(e)
        }
    }
}
fn publish_lamp(app: &tauri::AppHandle, result: &std::result::Result<LampState, String>) {
    let row = match result {
        Ok(state) => {
            let mut v = serde_json::to_value(state).unwrap();
            v["stale"] = json!(false);
            v["connection"] = json!("connected");
            v["lastSeen"] = v["sampledAt"].clone();
            cache_lamp(app, &v)
        }
        Err(error) => cache_lamp(
            app,
            &json!({"stale":true,"connection":"unknown","error":error}),
        ),
    };
    let _ = app.emit("lamp-update", row);
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LampConfig {
    configured: bool,
    ip: Option<String>,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LampState {
    power: bool,
    brightness: u32,
    temperature: u32,
    sampled_at: u64,
}
#[derive(Deserialize)]
struct LampChange {
    power: Option<bool>,
    brightness: Option<u32>,
    temperature: Option<u32>,
}
impl LampChange {
    fn validate(&self) -> Result<()> {
        if self.power.is_none() && self.brightness.is_none() && self.temperature.is_none() {
            return Err(message("请选择调节项"));
        }
        if self.brightness.is_some_and(|v| !(1..=100).contains(&v)) {
            return Err(message("亮度范围 1–100"));
        }
        if self
            .temperature
            .is_some_and(|v| !(2700..=6500).contains(&v))
        {
            return Err(message("色温范围 2700–6500 K"));
        }
        Ok(())
    }
}
fn load(backend: &Backend) -> Result<config::Device> {
    let encrypted = std::fs::read(&backend.file)?;
    let mut plain = crypto::protect(&encrypted, true)?;
    let result = serde_json::from_slice(&plain).map_err(Error::from);
    plain.zeroize();
    result
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
fn get_lamp_config(backend: State<Backend>) -> std::result::Result<LampConfig, String> {
    if !backend.file.exists() {
        return Ok(LampConfig {
            configured: false,
            ip: None,
        });
    }
    load(&backend)
        .map(|d| LampConfig {
            configured: true,
            ip: Some(d.ip.clone()),
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_lamp_config(
    ip: String,
    mut token: String,
    backend: State<Backend>,
) -> std::result::Result<LampConfig, String> {
    let result = (|| -> Result<LampConfig> {
        let _guard = backend
            .lamp_queue
            .lock()
            .map_err(|_| message("设备队列不可用"))?;
        let device = config::Device {
            ip: config::normalize_ip(&ip)?,
            token: token.clone(),
            did: None,
        };
        device.token_bytes()?;
        let mut plain = serde_json::to_vec(&device)?;
        let encrypted = crypto::protect(&plain, false);
        plain.zeroize();
        let encrypted = encrypted?;
        if let Some(parent) = backend.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pending = backend.file.with_extension("pending");
        std::fs::write(&pending, encrypted)?;
        // Same-directory rename: Windows replacement is handled explicitly.
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };
            let from: Vec<u16> = pending.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = backend
                .file
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            if unsafe {
                MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(pending, &backend.file)?;
        Ok(LampConfig {
            configured: true,
            ip: Some(device.ip.clone()),
        })
    })();
    token.zeroize();
    result.map_err(|e| e.to_string())
}
fn parse_state(value: &Value) -> Result<LampState> {
    let rows = value
        .as_array()
        .ok_or_else(|| message("挂灯回复格式无效"))?;
    let power = match rows.first().and_then(Value::as_str) {
        Some("on") => true,
        Some("off") => false,
        _ => return Err(message("未取得开关状态；当前仅支持 legacy get_prop 型号")),
    };
    let number = |index: usize| -> Result<u32> {
        rows.get(index)
            .and_then(|v| {
                v.as_u64()
                    .and_then(|x| u32::try_from(x).ok())
                    .or_else(|| v.as_str()?.parse().ok())
            })
            .ok_or_else(|| message("挂灯数值缺失"))
    };
    let brightness = number(1)?;
    let temperature = number(2)?;
    LampChange {
        power: None,
        brightness: Some(brightness),
        temperature: Some(temperature),
    }
    .validate()?;
    Ok(LampState {
        power,
        brightness,
        temperature,
        sampled_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    })
}
fn lamp_operation(backend: &Backend, change: Option<LampChange>) -> Result<LampState> {
    if let Some(ref change) = change {
        change.validate()?;
    }
    let _guard = backend
        .lamp_queue
        .lock()
        .map_err(|_| message("设备队列不可用"))?;
    let mut device = load(backend)?;
    device.did = Some(protocol::probe(&device.ip)?);
    let mut session = protocol::Session::new(&device)?;
    // Validate this protocol and state before enabling writes.
    let (before, miot) = match session.call("get_prop", json!(["power", "bright", "ct"])) {
        Ok(value) => match parse_state(&value) {
            Ok(s) => (s, false),
            Err(_) => (miot_state(&mut session, device.did.unwrap())?, true),
        },
        Err(Error::Device(-32601 | -4001)) => {
            (miot_state(&mut session, device.did.unwrap())?, true)
        }
        Err(e) => return Err(e),
    };
    if let Some(change) = change {
        if miot {
            let mut rows = Vec::new();
            for (piid, value) in [
                (1, change.power.map(Value::Bool)),
                (2, change.brightness.map(|v| json!(v))),
                (3, change.temperature.map(|v| json!(v))),
            ] {
                if let Some(value) = value {
                    rows.push(json!({"did":device.did.unwrap().to_string(),"siid":2,"piid":piid,"value":value}));
                }
            }
            let reply = session.call("set_properties", json!(rows))?;
            let results = reply
                .as_array()
                .ok_or_else(|| message("MIoT 设置回复无效"))?;
            if results.len() != rows.len()
                || rows.iter().any(|r| {
                    !results
                        .iter()
                        .any(|v| v["siid"] == r["siid"] && v["piid"] == r["piid"] && v["code"] == 0)
                })
            {
                return Err(message("MIoT 未确认全部属性设置"));
            }
            return confirm_change(&change, || miot_state(&mut session, device.did.unwrap()));
        }
        let mut apply = |method, params| -> Result<()> {
            let reply = session.call(method, params)?;
            if reply != json!(["ok"]) {
                return Err(message("设备未确认设置成功"));
            }
            Ok(())
        };
        if let Some(v) = change.power {
            apply(
                "set_power",
                json!([if v { "on" } else { "off" }, "smooth", 200]),
            )?;
        }
        if let Some(v) = change.brightness {
            apply("set_bright", json!([v, "smooth", 200]))?;
        }
        if let Some(v) = change.temperature {
            apply("set_ct_abx", json!([v, "smooth", 200]))?;
        }
        return confirm_change(&change, || {
            parse_state(&session.call("get_prop", json!(["power", "bright", "ct"]))?)
        });
    }
    Ok(before)
}
fn confirm_change(
    change: &LampChange,
    mut read: impl FnMut() -> Result<LampState>,
) -> Result<LampState> {
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let state = read()?;
        if change.power.is_none_or(|v| v == state.power)
            && change.brightness.is_none_or(|v| v == state.brightness)
            && change.temperature.is_none_or(|v| v == state.temperature)
        {
            return Ok(state);
        }
    }
    Err(message("设备已应答，但读回值尚未达到请求值，请刷新核对"))
}
fn miot_state(session: &mut impl Rpc, did: u32) -> Result<LampState> {
    let info = session.call("miIO.info", json!([]))?;
    if info["model"] != "yeelink.light.lamp22" {
        return Err(message(
            "当前仅有 lamp22 的 MIoT 主灯映射，未知型号禁止写入",
        ));
    }
    let props: Vec<Value> = (1..=3)
        .map(|piid| json!({"did":did.to_string(),"siid":2,"piid":piid}))
        .collect();
    let result = session.call("get_properties", json!(props))?;
    let rows = result
        .as_array()
        .ok_or_else(|| message("MIoT 状态格式无效"))?;
    let get = |piid| -> Result<Value> {
        rows.iter()
            .find(|v| v["siid"] == 2 && v["piid"] == piid && v["code"] == 0)
            .and_then(|v| v.get("value"))
            .cloned()
            .ok_or_else(|| message("MIoT 属性缺失或读取失败"))
    };
    let power = get(1)?
        .as_bool()
        .ok_or_else(|| message("MIoT 开关状态无效"))?;
    parse_state(&json!([if power { "on" } else { "off" }, get(2)?, get(3)?]))
}
#[tauri::command]
async fn read_lamp(app: tauri::AppHandle) -> std::result::Result<LampState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = lamp_operation(&app.state::<Backend>(), None).map_err(|e| e.to_string());
        publish_lamp(&app, &result);
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_lamp(
    change: LampChange,
    app: tauri::AppHandle,
) -> std::result::Result<LampState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result =
            lamp_operation(&app.state::<Backend>(), Some(change)).map_err(|e| e.to_string());
        publish_lamp(&app, &result);
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HidDevice {
    id: String,
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage: u16,
    interface_number: i32,
    manufacturer: Option<String>,
    product: Option<String>,
    battery_supported: bool,
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
async fn list_hid_devices() -> std::result::Result<Vec<HidDevice>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        Ok(api
            .device_list()
            .map(|d| HidDevice {
                id: hex::encode(d.path().to_bytes()),
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                interface_number: d.interface_number(),
                manufacturer: d.manufacturer_string().map(str::to_owned),
                product: d.product_string().map(str::to_owned),
                battery_supported: battery_match(d),
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn read_battery(
    device_id: String,
    app: tauri::AppHandle,
) -> std::result::Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = query_battery_sync(&device_id);
        let sample = match &result {
            Ok(v) => {
                let mut v = v.clone();
                v["stale"] = json!(false);
                v["connection"] = json!("connected");
                v
            }
            Err(e) => json!({"stale":true,"connection":"unknown","error":e}),
        };
        let row = cache_device(&app, &device_id, sample);
        let _ = app.emit("battery-update", row.clone());
        result.map(|_| row)
    })
    .await
    .map_err(|e| e.to_string())?
}
pub(crate) fn device_queue(id: &str) -> Arc<Mutex<()>> {
    static QUEUES: OnceLock<Mutex<std::collections::HashMap<String, std::sync::Weak<Mutex<()>>>>> =
        OnceLock::new();
    let mut queues = QUEUES
        .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    queues.retain(|_, v| v.strong_count() > 0);
    if let Some(queue) = queues.get(id).and_then(|q| q.upgrade()) {
        return queue;
    }
    let queue = Arc::new(Mutex::new(()));
    queues.insert(id.into(), Arc::downgrade(&queue));
    queue
}
fn kx_candidate(d: &hidapi::DeviceInfo) -> bool {
    d.vendor_id() == 0x3554
        && d.product_id() == 0xfa09
        && d.usage_page() == 0xff02
        && d.usage() == 2
        && d.interface_number() == 1
}
pub(crate) fn verify_kx75(device: &hidapi::HidDevice) -> std::result::Result<(), String> {
    let mut request = [0u8; 20];
    request[0] = 0x13;
    request[1] = 5;
    request[2] = 1;
    request[19] = 0x19;
    if device.write(&request).map_err(|e| e.to_string())? != request.len() {
        return Err("型号查询发送不完整".into());
    }
    // Official Yat.getPassWord waits up to 2200 ms; send this request only once.
    let response = read_kx_reply_with_timeout(device, 5, std::time::Duration::from_millis(2200))?;
    let frame = response.as_slice();
    if frame.len() != 20
        || frame[0] != 0x13
        || frame[1] != 5
        || frame[2] != 1
        || frame[3] != 0
        || frame[5..11] != [3, 0, 0, 0, 1, 0xc7]
        || frame[..19].iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) != frame[19]
    {
        return Err("只读型号探测未确认 KX75，禁止后续查询或设置".into());
    }
    Ok(())
}
pub(crate) fn read_kx_reply(
    device: &hidapi::HidDevice,
    command: u8,
) -> std::result::Result<Vec<u8>, String> {
    read_kx_reply_with_timeout(device, command, std::time::Duration::from_millis(500))
}
fn read_kx_reply_with_timeout(
    device: &hidapi::HidDevice,
    command: u8,
    timeout: std::time::Duration,
) -> std::result::Result<Vec<u8>, String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("KX75控制响应超时".into());
        }
        let mut buffer = [0u8; 64];
        let n = device
            .read_timeout(&mut buffer, remaining.as_millis().max(1) as i32)
            .map_err(|e| e.to_string())?;
        if n < 2 || buffer[0] != 0x13 || buffer[1] != command {
            continue;
        }
        #[cfg(test)]
        println!(
            "KX vendor response command={command:02X} bytes={}",
            hex::encode(&buffer[..n])
        );
        return Ok(buffer[..n].to_vec());
    }
}
pub(crate) fn query_battery_sync(device_id: &str) -> std::result::Result<Value, String> {
    let queue = device_queue(device_id);
    let _guard = queue.lock().map_err(|_| "HID 队列不可用")?;
    match query_battery_once(device_id) {
        Err(error) if battery_timeout(&error) => {
            std::thread::sleep(std::time::Duration::from_millis(150));
            query_battery_once(device_id)
        }
        result => result,
    }
}
fn battery_timeout(error: &str) -> bool {
    error.contains("超时")
        || error.contains("电量回复不足")
        || error.to_ascii_lowercase().contains("timed out")
}
fn query_battery_once(device_id: &str) -> std::result::Result<Value, String> {
    // Enumerate again: never open an arbitrary path supplied by the UI.
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let info = api
        .device_list()
        .find(|d| hex::encode(d.path().to_bytes()) == device_id && battery_match(d))
        .ok_or("设备不存在或协议尚未匹配，禁止发送查询")?;
    let device = info.open_device(&api).map_err(|e| e.to_string())?;
    if info.vendor_id() == 0x3554 {
        verify_kx75(&device)?;
        let mut request = [0u8; 20];
        request[0] = 0x13;
        request[1] = 0x4a;
        request[2] = 1;
        request[19] = 0x5e;
        if device.write(&request).map_err(|e| e.to_string())? != request.len() {
            return Err("KX75 查询发送不完整".into());
        }
        return parse_kx75(&read_kx_reply(&device, 0x4a)?).map_err(|e| e.to_string());
    }
    let mut request = [0u8; 17];
    request[0] = 0x08;
    request[1] = 0x04;
    request[16] = 0x49;
    if device.write(&request).map_err(|e| e.to_string())? != request.len() {
        return Err("HID 电量请求发送不完整".into());
    }
    let mut response = [0u8; 64];
    let count = device
        .read_timeout(&mut response, 500)
        .map_err(|e| e.to_string())?;
    parse_battery(&response[..count]).map_err(|e| e.to_string())
}
pub(crate) fn battery_match(d: &hidapi::DeviceInfo) -> bool {
    (d.vendor_id() == 0x373b
        && d.product_id() == 0x101a
        && d.usage_page() == 0xff02
        && d.usage() == 2
        && d.interface_number() == 1)
        || kx_candidate(d)
}
fn parse_kx75(response: &[u8]) -> Result<Value> {
    if response.len() != 20
        || response[0] != 0x13
        || response[1] != 0x4a
        || response[2] != 1
        || response[3] != 0
        || response[5] > 100
    {
        return Err(message("KX75 回复格式不匹配或电量无效"));
    }
    if response[..19]
        .iter()
        .fold(0u8, |sum, b| sum.wrapping_add(*b))
        != response[19]
    {
        return Err(message("KX75 回复校验失败"));
    }
    Ok(
        json!({"batteryPercent":response[5],"charging":null,"chargeStatusRaw":response[6]&15,"fullStatusRaw":response[6]>>4,
        "sampledAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
        "model":"MCHOSE KX75", "verifiedOnHardware":false}),
    )
}
fn parse_battery(response: &[u8]) -> Result<Value> {
    if response.len() < 10 {
        return Err(message("接收器超时或电量回复不足10字节"));
    }
    if response[6] > 100 || response[7] > 1 {
        return Err(message("电量或充电状态无效"));
    }
    Ok(
        json!({"batteryPercent":response[6], "charging":response[7] == 1,
        "voltageMv":u16::from_be_bytes([response[8],response[9]]),
        "sampledAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
        "model":"ATK Mouse 1K Dongle", "verifiedOnHardware":false}),
    )
}
#[tauri::command]
fn open_official(brand: String) -> std::result::Result<(), String> {
    let url = match brand.as_str() {
        "atk" => "https://hub.atk.pro/",
        "mchose" => "https://www.mchose.com.cn/#/connectDevice",
        _ => return Err("未知品牌".into()),
    };
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("仅支持 Windows".into())
    }
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
fn get_autostart(app: tauri::AppHandle) -> std::result::Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
async fn get_device_settings(
    device_id: String,
) -> std::result::Result<settings::SettingsSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::read_sync(&device_id))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED".into(),
            message: "设置查询任务异常".into(),
        })?
}
#[tauri::command]
async fn apply_device_settings(
    device_id: String,
    change: settings::SettingsChange,
) -> std::result::Result<settings::SettingsSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::apply_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED".into(),
            message: "设置应用任务异常".into(),
        })?
}
#[tauri::command]
async fn read_atk_performance(
    device_id: String,
) -> std::result::Result<settings::SettingsSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::read_performance_sync(&device_id))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "鼠标性能查询任务异常".into(),
        })?
}
#[tauri::command]
async fn apply_atk_performance(
    device_id: String,
    change: settings::atk_extensions::PerformanceChange,
) -> std::result::Result<settings::SettingsSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::apply_performance_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "鼠标性能应用任务异常".into(),
        })?
}
#[tauri::command]
async fn read_atk_dpi(
    device_id: String,
) -> std::result::Result<settings::atk_extensions::DpiSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::read_dpi_sync(&device_id))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "DPI查询任务异常".into(),
        })?
}
#[tauri::command]
async fn apply_atk_dpi(
    device_id: String,
    change: settings::atk_extensions::DpiChange,
) -> std::result::Result<settings::atk_extensions::DpiSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::apply_dpi_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "DPI应用任务异常".into(),
        })?
}
#[tauri::command]
async fn read_atk_macro(
    device_id: String,
    slot: u8,
) -> std::result::Result<settings::atk_extensions::MacroSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::read_macro_sync(&device_id, slot))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "宏查询任务异常".into(),
        })?
}
#[tauri::command]
async fn apply_atk_macro(
    device_id: String,
    change: settings::atk_extensions::MacroChange,
) -> std::result::Result<settings::atk_extensions::MacroSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::apply_macro_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "宏应用任务异常".into(),
        })?
}
#[tauri::command]
async fn bind_atk_macro(
    device_id: String,
    change: settings::atk_extensions::MacroBindingChange,
) -> std::result::Result<settings::atk_extensions::MacroSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::atk_extensions::bind_macro_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "宏绑定任务异常".into(),
        })?
}
#[tauri::command]
async fn read_button_mappings(
    device_id: String,
) -> std::result::Result<settings::mappings::MappingSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::mappings::read_sync(&device_id))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "按键映射查询任务异常".into(),
        })?
}
#[tauri::command]
async fn read_keyboard_lighting(
    device_id: String,
) -> std::result::Result<settings::kx_extensions::Snapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::kx_extensions::read_sync(&device_id))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "键盘灯效查询任务异常".into(),
        })?
}
#[tauri::command]
async fn apply_keyboard_lighting(
    device_id: String,
    change: settings::kx_extensions::Change,
) -> std::result::Result<settings::kx_extensions::Snapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::kx_extensions::apply_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "键盘灯效应用任务异常".into(),
        })?
}
#[tauri::command]
async fn read_keyboard_mappings(
    device_id: String,
    layer: String,
) -> std::result::Result<settings::kx_mapping::Snapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || {
        settings::kx_mapping::read_sync(&device_id, &layer)
    })
    .await
    .map_err(|_| settings::SettingsError {
        code: "TASK_FAILED",
        message: "键盘映射查询任务异常".into(),
    })?
}
#[tauri::command]
async fn apply_keyboard_mapping(
    device_id: String,
    change: settings::kx_mapping::Change,
) -> std::result::Result<settings::kx_mapping::Snapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || {
        settings::kx_mapping::apply_sync(&device_id, change)
    })
    .await
    .map_err(|_| settings::SettingsError {
        code: "TASK_FAILED",
        message: "键盘映射应用任务异常".into(),
    })?
}
#[tauri::command]
async fn apply_button_mapping(
    device_id: String,
    change: settings::mappings::MappingChange,
) -> std::result::Result<settings::mappings::MappingSnapshot, settings::SettingsError> {
    tauri::async_runtime::spawn_blocking(move || settings::mappings::apply_sync(&device_id, change))
        .await
        .map_err(|_| settings::SettingsError {
            code: "TASK_FAILED",
            message: "按键映射应用任务异常".into(),
        })?
}
#[tauri::command]
fn set_autostart(enabled: bool, app: tauri::AppHandle) -> std::result::Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())?;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
#[tauri::command]
async fn show_main_window(app: tauri::AppHandle) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || show_main_window_sync(app))
        .await
        .map_err(|e| e.to_string())?
}
fn show_main_window_sync(app: tauri::AppHandle) -> std::result::Result<(), String> {
    let window = if let Some(window) = app.get_webview_window("main") {
        window
    } else {
        tauri::WebviewWindowBuilder::from_config(&app, &app.config().app.windows[0])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?
    };
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn close_device_window(window: tauri::WebviewWindow) -> std::result::Result<(), String> {
    if window.label() == "main" {
        cloud::cancel_all(window.app_handle())?;
    }
    window.destroy().map_err(|e| e.to_string())
}
#[tauri::command]
fn exit_device_center(app: tauri::AppHandle) -> std::result::Result<(), String> {
    cloud::cancel_all(&app)?;
    for id in ["battery-mouse", "battery-keyboard", "logo"] {
        drop(app.remove_tray_by_id(id));
    }
    app.exit(0);
    Ok(())
}
fn request_tray_exit(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit_to("main", "request-window-close", json!({"exit":true}));
    } else if let Some(window) = app.get_webview_window("compact") {
        let _ = window.emit_to("compact", "request-compact-exit", ());
    } else {
        let _ = exit_device_center(app.clone());
    }
}
fn show_compact_window(
    app: &tauri::AppHandle,
    anchor: Option<tauri::Rect>,
) -> std::result::Result<(), String> {
    static PRESENT: Mutex<()> = Mutex::new(());
    let _present = PRESENT.lock().map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("compact") {
        if window.is_visible().unwrap_or(false) {
            return window.emit_to("compact", "request-window-close", json!({"exit":false})).map_err(|e| e.to_string());
        }
    }
    let anchor = anchor.or_else(|| app.tray_by_id("logo").and_then(|tray| tray.rect().ok().flatten()));
    let point = anchor.as_ref().map(|rect| rect.position.to_physical::<i32>(1.));
    let monitor = if let Some(point) = point {
        app.monitor_from_point(point.x as f64, point.y as f64).map_err(|e|e.to_string())?
    } else { app.primary_monitor().map_err(|e|e.to_string())? }
    .ok_or("找不到快捷窗所在显示器")?;
    let scale = monitor.scale_factor();
    let window = if let Some(window) = app.get_webview_window("compact") {
        window
    } else {
        let builder = tauri::WebviewWindowBuilder::new(
            app,
            "compact",
            tauri::WebviewUrl::App("index.html?view=compact".into()),
        )
        .title("设备快捷控制")
        .inner_size(compact_position::WIDTH, compact_position::HEIGHT)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false);
        #[cfg(feature = "lifecycle-validation")]
        let builder = builder.data_directory(lifecycle_validation::root().join("webview-profile"));
        builder.build().map_err(|e| e.to_string())?
    };
    let work = monitor.work_area();
    let work = compact_position::Rect { x:work.position.x, y:work.position.y, width:work.size.width, height:work.size.height };
    let anchor = anchor.map(|rect| {
        let point = rect.position.to_physical::<i32>(scale);
        let size = rect.size.to_physical::<u32>(scale);
        compact_position::Rect { x:point.x, y:point.y, width:size.width, height:size.height }
    }).unwrap_or(compact_position::Rect { x:work.x + work.width as i32, y:work.y + work.height as i32, width:0, height:0 });
    let gap = (8. * scale).round() as i32;
    let requested = tauri::LogicalSize::new(compact_position::WIDTH, compact_position::HEIGHT).to_physical::<u32>(scale);
    let (x,y) = compact_position::place(anchor,work,(requested.width,requested.height),gap);
    // Move onto the target monitor before sizing, then use the actual native
    // outer size for the final position. Never show a centered intermediate frame.
    window.set_position(tauri::PhysicalPosition::new(x,y)).map_err(|e|e.to_string())?;
    window.set_size(requested).map_err(|e|e.to_string())?;
    let size = window.outer_size().map_err(|e|e.to_string())?;
    let (x,y) = compact_position::place(anchor,work,(size.width,size.height),gap);
    window.set_position(tauri::PhysicalPosition::new(x,y)).map_err(|e|e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(feature = "lifecycle-validation")]
    let context = lifecycle_validation::isolated_context(context);
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::Builder::new().build());
    #[cfg(not(feature = "lifecycle-validation"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_lamp_config,
        save_lamp_config,
        read_lamp,
        set_lamp,
        list_hid_devices,
        read_battery,
        open_official,
        get_autostart,
        set_autostart,
        get_device_settings,
        apply_device_settings,
        read_atk_performance,
        apply_atk_performance,
        read_atk_dpi,
        apply_atk_dpi,
        read_atk_macro,
        apply_atk_macro,
        bind_atk_macro,
        read_button_mappings,
        apply_button_mapping,
        read_keyboard_mappings,
        apply_keyboard_mapping,
        read_keyboard_lighting,
        apply_keyboard_lighting,
        get_device_snapshots,
        show_main_window,
        close_device_window,
        exit_device_center,
        tray_display::get_tray_preferences,
        tray_display::set_tray_preferences,
        cloud::begin_mi_login,
        cloud::poll_mi_login,
        cloud::list_mi_devices,
        cloud::import_mi_lamp,
        cloud::cancel_mi_login
    ]);
    #[cfg(feature = "lifecycle-validation")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        lifecycle_validation::get_lamp_config,
        lifecycle_validation::list_hid_devices,
        lifecycle_validation::get_autostart,
        lifecycle_validation::get_device_settings,
        cloud::lifecycle_mock::begin_mi_login,
        cloud::lifecycle_mock::poll_mi_login,
        cloud::cancel_mi_login,
        lifecycle_validation::lifecycle_test_log,
        get_device_snapshots,
        show_main_window,
        close_device_window,
        exit_device_center
    ]);
    builder
        .setup(|app| {
            app.manage(Backend {
                file: {
                    #[cfg(feature = "lifecycle-validation")]
                    {
                        lifecycle_validation::root().join("empty-config/lamp.dpapi")
                    }
                    #[cfg(not(feature = "lifecycle-validation"))]
                    {
                        app.path().app_config_dir()?.join("lamp.dpapi")
                    }
                },
                lamp_queue: Mutex::new(()),
            });
            app.manage(cloud::CloudState::default());
            app.manage(SnapshotStore::default());
            #[cfg(not(feature = "lifecycle-validation"))]
            monitor::start(app.handle().clone());
            #[cfg(feature = "lifecycle-validation")]
            lifecycle_validation::start(app.handle().clone());
            use tauri::{
                menu::{Menu, MenuItem, PredefinedMenuItem},
                tray::TrayIconBuilder,
            };
            let show = MenuItem::with_id(app, "show", "打开设备中心", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let compact = MenuItem::with_id(app, "compact", "快捷控制", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&compact, &show, &separator, &quit])?;
            let pixels = include_bytes!("../../assets/icons-v2/tray-32.rgba").to_vec();
            TrayIconBuilder::with_id("logo")
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("设备中心")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle().clone();
                        std::thread::spawn(move || {
                            let _ = show_compact_window(&app, Some(rect));
                        });
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let app = app.clone();
                        std::thread::spawn(move || {
                            let _ = show_main_window_sync(app);
                        });
                    }
                    "compact" => {
                        let app = app.clone();
                        std::thread::spawn(move || {
                            let _ = show_compact_window(&app, None);
                        });
                    }
                    "quit" => {
                        request_tray_exit(app);
                    }
                    _ => (),
                })
                .build(app)?;
            #[cfg(not(feature = "lifecycle-validation"))]
            tray_display::start(app.handle())?;
            Ok(())
        })
        .on_page_load(|webview, payload| {
            #[cfg(feature = "lifecycle-validation")]
            lifecycle_validation::page(webview, payload);
            #[cfg(not(feature = "lifecycle-validation"))]
            let _ = (webview, payload);
        })
        .on_window_event(|window, event| {
            #[cfg(feature = "lifecycle-validation")]
            if matches!(event, tauri::WindowEvent::Destroyed) {
                lifecycle_validation::record(&format!("destroyed_{}", window.label()));
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "compact" {
                    let _ =
                        window.emit_to("compact", "request-window-close", json!({"exit":false}));
                } else {
                    let _ = window.emit_to("main", "request-window-close", json!({"exit":false}));
                }
            }
            if window.label() == "compact" && matches!(event, tauri::WindowEvent::Focused(false)) {
                let window = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    if window.is_visible().unwrap_or(false) && !window.is_focused().unwrap_or(false)
                    {
                        let _ = window.emit_to(
                            "compact",
                            "request-window-close",
                            json!({"exit":false}),
                        );
                    }
                });
            }
        })
        .build(context)
        .expect("无法启动设备中心")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_share_only_their_own_device_queue() {
        let first = device_queue("test-path-a");
        let same = device_queue("test-path-a");
        let other = device_queue("test-path-b");
        assert!(Arc::ptr_eq(&first, &same));
        assert!(!Arc::ptr_eq(&first, &other));
        let _guard = first.lock().unwrap();
        assert!(same.try_lock().is_err());
        assert!(other.try_lock().is_ok());
    }
    #[test]
    fn snapshot_error_preserves_last_value_and_success_clears_error() {
        let mut row = json!({"batteryPercent":55,"sampledAt":1,"stale":false});
        merge_snapshot(&mut row, &json!({"stale":true,"error":"timeout"}));
        let before = row["updatedAt"].as_u64().unwrap();
        assert_eq!(row["batteryPercent"], 55);
        assert_eq!(row["sampledAt"], 1);
        merge_snapshot(
            &mut row,
            &json!({"stale":false,"batteryPercent":54,"sampledAt":2}),
        );
        assert!(row.get("error").is_none());
        assert!(row["updatedAt"].as_u64().unwrap() > before);
    }
    #[test]
    fn kx75_single_packet_sequence_and_checksum() {
        // Constructed protocol fixture, not a captured device response.
        let mut frame = [0u8; 20];
        frame[0] = 0x13;
        frame[1] = 0x4a;
        frame[2] = 1;
        frame[5] = 75;
        frame[19] = frame[..19].iter().fold(0u8, |sum, b| sum.wrapping_add(*b));
        assert_eq!(parse_kx75(&frame).unwrap()["batteryPercent"], 75);
        frame[3] = 1;
        assert!(parse_kx75(&frame).is_err());
        frame[3] = 0;
        frame[19] ^= 1;
        assert!(parse_kx75(&frame).is_err());
    }
    #[test]
    fn acknowledged_but_stale_lamp_is_failure() {
        let change = LampChange {
            power: None,
            brightness: Some(80),
            temperature: None,
        };
        assert!(confirm_change(&change, || parse_state(&json!(["on", 50, 4000]))).is_err());
        assert!(confirm_change(&change, || parse_state(&json!(["on", 80, 4000]))).is_ok());
    }
    #[test]
    #[ignore = "Explicit hardware-only battery query; never part of default test run"]
    fn hardware_read_once() {
        let api = hidapi::HidApi::new().unwrap();
        for d in api
            .device_list()
            .filter(|d| d.vendor_id() == 0x3554 && d.product_id() == 0xfa09)
        {
            println!("KX candidate VID=3554 PID=FA09 usagePage={:04X} usage={} interface={} product={:?} allowed={}",d.usage_page(),d.usage(),d.interface_number(),d.product_string(),battery_match(d));
        }
        for d in api.device_list().filter(|d| {
            d.vendor_id() == 0x3554
                && d.product_id() == 0xfa09
                && d.usage_page() == 0xff02
                && d.usage() == 2
                && d.interface_number() == 1
        }) {
            println!(
                "KX FF02 sibling read-only model probe={:?}",
                d.open_device(&api)
                    .map_err(|e| e.to_string())
                    .and_then(|device| verify_kx75(&device))
            );
        }
        let matches: Vec<_> = api.device_list().filter(|d| battery_match(d)).collect();
        println!("matched interfaces: {}", matches.len());
        for d in matches {
            println!(
                "VID={:04X} PID={:04X} usagePage={:04X} usage={} interface={} result={:?}",
                d.vendor_id(),
                d.product_id(),
                d.usage_page(),
                d.usage(),
                d.interface_number(),
                query_battery_sync(&hex::encode(d.path().to_bytes()))
            );
            let settings_result = if d.vendor_id() == 0x373b {
                settings::diagnostic_read_sync(&hex::encode(d.path().to_bytes()))
            } else {
                settings::read_sync(&hex::encode(d.path().to_bytes()))
            };
            match settings_result {
                Ok(snapshot) => {
                    let mut value = serde_json::to_value(snapshot).unwrap();
                    if let Some(row) = value.as_object_mut() {
                        row.remove("deviceId");
                    }
                    println!("settings={value}");
                }
                Err(error) => println!("settings_error={}", serde_json::to_value(error).unwrap()),
            }
        }
    }
    #[test]
    fn limits_and_unknown_state() {
        assert!(battery_timeout("KX75控制响应超时"));
        assert!(!battery_timeout(
            "只读型号探测未确认 KX75，禁止后续查询或设置"
        ));
        assert!(!battery_timeout("KX75 回复校验失败"));
        assert!(parse_state(&json!(["on", 50, 4000])).is_ok());
        for v in [
            json!([]),
            json!(["unknown", 50, 4000]),
            json!(["on", 0, 4000]),
            json!(["on", 50, 99999]),
        ] {
            assert!(parse_state(&v).is_err());
        }
        assert!(config::normalize_ip("8.8.8.8").is_err());
        assert!(config::normalize_ip("192.168.1.2").is_ok());
        assert!(parse_battery(&[0; 9]).is_err());
        let mut frame = [0u8; 10];
        frame[6] = 75;
        frame[7] = 1;
        assert_eq!(parse_battery(&frame).unwrap()["batteryPercent"], 75);
        frame[6] = 101;
        assert!(parse_battery(&frame).is_err());
    }
}
