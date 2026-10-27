//! Xiaomi QR login and device-list protocol adapted from QiLight cloud.rs (GPL-3.0).
//! Never returns miIO/service tokens to the WebView and never logs cloud responses.
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::{
    blocking::{Client, Response},
    cookie::{CookieStore, Jar},
    redirect::Policy,
    Url,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};
use zeroize::{Zeroize, Zeroizing};
type Result<T> = std::result::Result<T, String>;
const MAX_BODY: u64 = 4 * 1024 * 1024;
const REGIONS: [&str; 7] = ["cn", "sg", "de", "us", "tw", "ru", "in"];
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn trusted(text: &str) -> Result<Url> {
    let u = Url::parse(text).map_err(|_| "小米登录地址无效")?;
    let ok = u.host_str().is_some_and(|h| {
        ["xiaomi.com", "mi.com"]
            .iter()
            .any(|d| h == *d || h.ends_with(&format!(".{d}")))
    });
    if !ok
        || u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some_and(|p| p != 443)
    {
        return Err("小米服务地址未通过安全校验".into());
    }
    Ok(u)
}
fn net<T>(r: std::result::Result<T, reqwest::Error>) -> Result<T> {
    r.map_err(|e| {
        if e.is_timeout() {
            "小米云端请求超时，请重试".into()
        } else {
            "小米云端连接失败，请检查网络后重试".into()
        }
    })
}
fn body(r: Response, limit: u64) -> Result<Vec<u8>> {
    if !r.status().is_success() {
        return Err(format!(
            "小米云端HTTP状态{}，请重试或重新扫码",
            r.status().as_u16()
        ));
    }
    if r.content_length().is_some_and(|n| n > limit) {
        return Err("小米服务响应过大".into());
    }
    let mut bytes = Vec::new();
    r.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "小米响应读取失败")?;
    if bytes.len() as u64 > limit {
        bytes.zeroize();
        return Err("小米服务响应过大".into());
    }
    Ok(bytes)
}
fn parse_json(text: &[u8]) -> Result<Value> {
    let t = text.strip_prefix(b"&&&START&&&").unwrap_or(text);
    serde_json::from_slice(t).map_err(|_| "小米响应格式无效，请重新扫码".into())
}
fn cloud_json(r: Response) -> Result<Value> {
    let b = Zeroizing::new(body(r, MAX_BODY)?);
    parse_json(&b)
}
fn id(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
fn b64(s: &str) -> Result<Vec<u8>> {
    BASE64
        .decode(s)
        .map_err(|_| "米家会话密钥无效，请重新扫码".into())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginStart {
    session_id: String,
    qr_image_data_url: String,
    expires_at: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginStatus {
    status: &'static str,
    expires_at: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceSummary {
    id: String,
    name: String,
    model: String,
    ip: Option<String>,
    eligible: bool,
    reason: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceList {
    devices: Vec<DeviceSummary>,
    fetched_at: u64,
    warnings: Vec<String>,
}
struct CloudDevice {
    summary: DeviceSummary,
    token: Zeroizing<String>,
}
struct Session {
    client: Client,
    jar: Arc<Jar>,
    region: String,
    cancel: Arc<AtomicBool>,
    qr: Value,
    deadline: Instant,
    expires_at: u64,
    authenticated: bool,
    security: Zeroizing<String>,
    service_token: Zeroizing<String>,
    user_id: Zeroizing<String>,
    devices: HashMap<String, CloudDevice>,
}
impl Session {
    fn new(region: &str, cancel: Arc<AtomicBool>) -> Result<Self> {
        if !REGIONS.contains(&region) {
            return Err("米家地区无效".into());
        }
        let jar = Arc::new(Jar::default());
        let client = net(Client::builder()
            .cookie_provider(jar.clone())
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent("DeviceCenter/0.1 APP/com.xiaomi.mihome APPV/10.5.201")
            .redirect(Policy::custom(|a| {
                if a.previous().len() >= 10 || trusted(a.url().as_str()).is_err() {
                    a.error("untrusted Xiaomi redirect")
                } else {
                    a.follow()
                }
            }))
            .build())?;
        Ok(Self {
            client,
            jar,
            region: region.into(),
            cancel,
            qr: Value::Null,
            deadline: Instant::now(),
            expires_at: 0,
            authenticated: false,
            security: Zeroizing::new(String::new()),
            service_token: Zeroizing::new(String::new()),
            user_id: Zeroizing::new(String::new()),
            devices: HashMap::new(),
        })
    }
    fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("米家登录已取消，请重新发起".into())
        } else if Instant::now() >= self.deadline {
            Err("米家登录会话已过期，请重新扫码".into())
        } else {
            Ok(())
        }
    }
    fn start(&mut self) -> Result<String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("米家登录已取消".into());
        }
        self.qr = cloud_json(net(self
            .client
            .get("https://account.xiaomi.com/longPolling/loginUrl")
            .query(&[
                ("_qrsize", "480"),
                ("qs", "%3Fsid%3Dxiaomiio%26_json%3Dtrue"),
                ("callback", "https://sts.api.io.mi.com/sts"),
                ("_hasLogo", "false"),
                ("sid", "xiaomiio"),
                ("serviceParam", ""),
                ("_locale", "zh_CN"),
                ("_dc", &now().to_string()),
            ])
            .send())?)?;
        let image = trusted(self.qr["qr"].as_str().ok_or("小米暂未生成二维码，请重试")?)?;
        trusted(self.qr["lp"].as_str().ok_or("小米登录轮询地址缺失")?)?;
        let seconds = self.qr["timeout"].as_u64().unwrap_or(180).clamp(1, 300);
        self.deadline = Instant::now() + Duration::from_secs(seconds);
        self.expires_at = now() + seconds * 1000;
        let bytes = body(net(self.client.get(image).send())?, 2 * 1024 * 1024)?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("小米二维码图片不是受支持的PNG格式".into());
        }
        self.check()?;
        Ok(format!("data:image/png;base64,{}", BASE64.encode(bytes)))
    }
    fn poll(&mut self) -> Result<LoginStatus> {
        self.check()?;
        if self.authenticated {
            return Ok(LoginStatus {
                status: "authenticated",
                expires_at: self.expires_at,
            });
        }
        let u = trusted(self.qr["lp"].as_str().ok_or("请先生成二维码")?)?;
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let response = match self
            .client
            .get(u)
            .timeout(remaining.min(Duration::from_secs(12)))
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                self.check()?;
                return Ok(LoginStatus {
                    status: "pending",
                    expires_at: self.expires_at,
                });
            }
            Err(_) => return Err("扫码状态连接失败，请重试".into()),
        };
        self.check()?;
        if response.status().as_u16() != 200 {
            return Err("扫码会话未获确认，请重新生成二维码".into());
        }
        let reply = cloud_json(response)?;
        let security = reply["ssecurity"]
            .as_str()
            .ok_or("二维码已过期、登录未确认或需要额外安全验证；请在米家完成验证后重试")?;
        let user = id(&reply["userId"]);
        if user.is_empty() || !user.bytes().all(|b| b.is_ascii_digit()) || b64(security)?.is_empty()
        {
            return Err("小米登录会话数据无效，请重新扫码".into());
        }
        let location = trusted(reply["location"].as_str().ok_or("登录确认地址缺失")?)?;
        let r = net(self.client.get(location.clone()).send())?;
        if !r.status().is_success() {
            return Err("小米登录验证未完成，请重新扫码".into());
        }
        let final_url = r.url().clone();
        drop(r);
        let mut token = None;
        for u in [
            location,
            final_url,
            Url::parse("https://api.io.mi.com/").unwrap(),
            Url::parse("https://sts.api.io.mi.com/").unwrap(),
        ] {
            if let Some(c) = self.jar.cookies(&u) {
                if let Ok(s) = c.to_str() {
                    if let Some(t) = s
                        .split(';')
                        .find_map(|x| x.trim().strip_prefix("serviceToken="))
                    {
                        token = Some(t.to_owned());
                        break;
                    }
                }
            }
        }
        let token = token
            .filter(|s| !s.is_empty() && !s.contains(['\r', '\n']))
            .ok_or("小米登录验证尚未完成，请重新扫码")?;
        *self.security = security.into();
        *self.user_id = user;
        *self.service_token = token;
        self.authenticated = true;
        self.deadline = Instant::now() + Duration::from_secs(900);
        self.expires_at = now() + 900_000;
        self.qr = Value::Null;
        self.check()?;
        Ok(LoginStatus {
            status: "authenticated",
            expires_at: self.expires_at,
        })
    }
    fn api(&self, path: &str, payload: Value) -> Result<Value> {
        self.check()?;
        if !self.authenticated {
            return Err("请先完成米家扫码确认".into());
        }
        let mut nonce = Vec::from(rand::random::<[u8; 8]>());
        nonce.extend_from_slice(&((now() / 60_000) as u32).to_be_bytes());
        let mut hash = Sha256::new();
        hash.update(b64(&self.security)?);
        hash.update(&nonce);
        let key = Zeroizing::new(hash.finalize().to_vec());
        let signed = Zeroizing::new(BASE64.encode(&*key));
        let sign = |fields: &[(String, String)]| {
            let mut terms = vec!["POST".to_owned(), path.into()];
            terms.extend(fields.iter().map(|(k, v)| format!("{k}={v}")));
            terms.push(signed.to_string());
            BASE64.encode(Sha1::digest(terms.join("&").as_bytes()))
        };
        let mut fields = vec![(
            "data".into(),
            serde_json::to_string(&payload).map_err(|_| "米家请求编码失败")?,
        )];
        fields.push(("rc4_hash__".into(), sign(&fields)));
        for (_, v) in &mut fields {
            let encrypted =
                crate::crypto::rc4(v.as_bytes(), &key, 1024).map_err(|_| "米家请求加密失败")?;
            v.zeroize();
            *v = BASE64.encode(encrypted);
        }
        fields.extend([
            ("signature".into(), sign(&fields)),
            ("ssecurity".into(), self.security.to_string()),
            ("_nonce".into(), BASE64.encode(nonce)),
        ]);
        let host = if self.region == "cn" {
            "api.io.mi.com".into()
        } else {
            format!("{}.api.io.mi.com", self.region)
        };
        let cookie=Zeroizing::new(format!("userId={}; serviceToken={}; yetAnotherServiceToken={}; locale=zh_CN; timezone=GMT+08:00; is_daylight=0; dst_offset=0; channel=MI_APP_STORE",*self.user_id,*self.service_token,*self.service_token));
        let response = net(self
            .client
            .post(format!("https://{host}/app{path}"))
            .form(&fields)
            .header("Cookie", cookie.as_str())
            .header("Accept-Encoding", "identity")
            .header("x-xiaomi-protocal-flag-cli", "PROTOCAL-HTTP2")
            .header("MIOT-ENCRYPT-ALGORITHM", "ENCRYPT-RC4")
            .send());
        for (_, v) in &mut fields {
            v.zeroize();
        }
        let encrypted = Zeroizing::new(body(response?, MAX_BODY)?);
        self.check()?;
        let bytes = b64(std::str::from_utf8(&encrypted)
            .map_err(|_| "米家响应编码无效")?
            .trim())?;
        let plain =
            Zeroizing::new(crate::crypto::rc4(&bytes, &key, 1024).map_err(|_| "米家响应解密失败")?);
        api_result(parse_json(&plain)?)
    }
    fn list(&mut self) -> Result<DeviceList> {
        self.check()?;
        self.devices.clear();
        let primary = self.list_v2();
        let (devices, warnings) = recover_listing(primary, || {
            self.api("/home/device_list",json!({"getVirtualModel":true,"getHuamiDevices":1,"get_split_device":false,"support_smart_home":true}))
        })?;
        self.check()?;
        let mut summaries: Vec<_> = devices.values().map(|d| d.summary.clone()).collect();
        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        self.devices = devices;
        Ok(DeviceList {
            devices: summaries,
            fetched_at: now(),
            warnings,
        })
    }
    fn list_v2(&self) -> Result<HashMap<String, CloudDevice>> {
        let homes = self.api(
            "/v2/homeroom/gethome",
            json!({"fg":true,"fetch_share":true,"fetch_share_dev":true,"limit":300,"app_ver":7}),
        )?;
        validate_homes(&homes)?;
        let mut visited = HashSet::new();
        let mut devices = HashMap::new();
        for home in ["homelist", "share_home_list"]
            .into_iter()
            .flat_map(|k| homes[k].as_array().into_iter().flatten())
        {
            let hid = id(&home["id"]);
            if hid.is_empty() {
                return Err(format!(
                    "米家庭列表行缺少有效标识（row:{}）",
                    type_name(home)
                ));
            }
            if !visited.insert(hid) {
                continue;
            }
            if visited.len() > 100 {
                return Err("米家庭列表超过本次查询上限".into());
            }
            let mut cursor = Value::Null;
            let mut cursors = HashSet::new();
            for page in 0..100 {
                self.check()?;
                let mut payload = json!({"home_owner":home.get("uid").cloned().unwrap_or(json!(*self.user_id)),"home_id":home["id"],"limit":200,"get_split_device":true,"support_smart_home":true});
                if !cursor.is_null() {
                    payload["start_did"] = cursor.clone();
                }
                let result = self.api("/v2/home/home_device_list", payload)?;
                let list = device_rows(&result)?;
                collect_devices(list, &mut devices)?;
                if !has_more(&result)? {
                    break;
                }
                cursor = result
                    .get("max_did")
                    .filter(|v| !v.is_null())
                    .cloned()
                    .or_else(|| list.last().and_then(|d| d.get("did").cloned()))
                    .unwrap_or(Value::Null);
                if page == 99 || cursor.is_null() || !cursors.insert(cursor.to_string()) {
                    return Err("米家设备分页异常，未保存任何配置".into());
                }
            }
        }
        self.check()?;
        Ok(devices)
    }
}
fn api_result(v: Value) -> Result<Value> {
    if v["code"].as_i64() != Some(0) {
        return Err(format!(
            "米家会话失效、权限不足或服务拒绝（code={}），请重新扫码核对账号和地区",
            v["code"]
                .as_i64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "非整数".into())
        ));
    }
    Ok(v["result"].clone())
}
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
/// Only fixed schema names and JSON types are emitted. Unknown/dynamic keys and all values stay private.
fn schema(v: &Value) -> String {
    const KEYS: [&str; 9] = [
        "code",
        "result",
        "homelist",
        "share_home_list",
        "device_info",
        "list",
        "has_more",
        "max_did",
        "devices",
    ];
    let Some(object) = v.as_object() else {
        return format!("root:{}", type_name(v));
    };
    let mut fields: Vec<String> = KEYS
        .iter()
        .filter_map(|k| object.get(*k).map(|v| format!("{k}:{}", type_name(v))))
        .collect();
    fields.push(format!(
        "other_fields:{}",
        object
            .keys()
            .filter(|k| !KEYS.contains(&k.as_str()))
            .count()
    ));
    fields.join(",")
}
fn validate_homes(v: &Value) -> Result<()> {
    let mut present = false;
    for key in ["homelist", "share_home_list"] {
        if let Some(value) = v.get(key) {
            if !value.is_array() && !value.is_null() {
                return Err(format!("米家庭列表结构不兼容（{}）", schema(v)));
            }
            present |= value.is_array();
        }
    }
    if !present {
        return Err(format!("米家庭列表字段缺失（{}）", schema(v)));
    }
    Ok(())
}
fn device_rows(v: &Value) -> Result<&Vec<Value>> {
    for key in ["device_info", "list"] {
        if let Some(rows) = v.get(key).and_then(Value::as_array) {
            return Ok(rows);
        }
    }
    Err(format!("米家设备列表结构不兼容（{}）", schema(v)))
}
fn has_more(v: &Value) -> Result<bool> {
    match v.get("has_more") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::Number(n)) if n.as_u64() == Some(0) => Ok(false),
        Some(Value::Number(n)) if n.as_u64() == Some(1) => Ok(true),
        _ => Err(format!("米家分页状态结构不兼容（{}）", schema(v))),
    }
}
fn recover_listing(
    primary: Result<HashMap<String, CloudDevice>>,
    fallback: impl FnOnce() -> Result<Value>,
) -> Result<(HashMap<String, CloudDevice>, Vec<String>)> {
    match primary {
        Ok(devices) => Ok((devices, Vec::new())),
        Err(primary) => {
            // Known alternative from MiLogin.ts; one fallback, not a silent empty substitution.
            let result = fallback().map_err(|error| {
                format!("家庭设备读取失败：{primary}；兼容设备读取也失败：{error}")
            })?;
            let rows = device_rows(&result).map_err(|error| {
                format!("家庭设备读取失败：{primary}；兼容设备读取也失败：{error}")
            })?;
            let mut devices = HashMap::new();
            collect_devices(rows, &mut devices)?;
            let mut warnings=vec![format!("已使用兼容设备列表；家庭分页读取未完成（{primary}）。该列表的共享家庭及分页覆盖未经验证，请核对目标挂灯。")];
            if has_more(&result)? {
                warnings.push(
                    "兼容接口提示仍有更多设备，当前列表可能不完整；未自动保存任何设备。".into(),
                );
            }
            Ok((devices, warnings))
        }
    }
}
fn collect_devices(rows: &[Value], devices: &mut HashMap<String, CloudDevice>) -> Result<()> {
    for row in rows {
        let d = parse_device(row)?.ok_or_else(|| {
            format!(
                "米家设备行缺少有效标识（row:{}），未导入该列表",
                type_name(row)
            )
        })?;
        devices.insert(d.summary.id.clone(), d);
        if devices.len() > 5000 {
            return Err("米家设备列表超过本次查询上限".into());
        }
    }
    Ok(())
}
fn parse_device(v: &Value) -> Result<Option<CloudDevice>> {
    let did = id(&v["did"]);
    if did.is_empty() {
        return Ok(None);
    }
    let model = v["model"].as_str().unwrap_or("").to_owned();
    let raw_ip = v["localip"]
        .as_str()
        .or_else(|| v["ip"].as_str())
        .unwrap_or("");
    let ip = crate::config::normalize_ip(raw_ip).ok();
    let token = Zeroizing::new(v["token"].as_str().unwrap_or("").to_owned());
    let valid_token = token.len() == 32
        && token.bytes().all(|c| c.is_ascii_hexdigit())
        && token.as_str() != "00000000000000000000000000000000";
    let reason = if model != "yeelink.light.lamp22" {
        Some("当前仅允许已支持的显示器挂灯型号yeelink.light.lamp22".into())
    } else if ip.is_none() {
        Some("云端未提供有效局域网IPv4，请使用手动配置".into())
    } else if !valid_token {
        Some("当前账号未提供有效设备Token，请核对家庭权限".into())
    } else {
        None
    };
    Ok(Some(CloudDevice {
        summary: DeviceSummary {
            id: did,
            name: v["name"].as_str().unwrap_or("未命名设备").into(),
            model,
            ip,
            eligible: reason.is_none(),
            reason,
        },
        token,
    }))
}
#[derive(Clone)]
struct Active {
    id: String,
    session: Arc<Mutex<Session>>,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub(crate) struct CloudState {
    active: Mutex<Option<Active>>,
}
impl CloudState {
    /// Window destruction must revoke the cloud session without waiting for an in-flight poll.
    pub(crate) fn cancel_all(&self) -> Result<()> {
        let mut state = self.active.lock().map_err(|_| "米家会话不可用")?;
        if let Some(active) = state.take() {
            active.cancel.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    fn get(&self, id: &str) -> Result<Active> {
        self.active
            .lock()
            .map_err(|_| "米家会话不可用")?
            .as_ref()
            .filter(|a| a.id == id)
            .cloned()
            .ok_or_else(|| "米家会话不存在或已结束，请重新扫码".into())
    }
    fn cancel(&self, id: &str) -> Result<()> {
        let mut state = self.active.lock().map_err(|_| "米家会话不可用")?;
        if let Some(a) = state.as_ref().filter(|a| a.id == id) {
            a.cancel.store(true, Ordering::Relaxed);
            *state = None;
        }
        Ok(())
    }
}
pub(crate) fn cancel_all(app: &tauri::AppHandle) -> Result<()> {
    app.state::<CloudState>().cancel_all()
}
// Holds only an opaque identifier and app handle while asleep, never a Session/secret.
fn expire_session(app: tauri::AppHandle, id: String) {
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = {
                let state = app.state::<CloudState>();
                let Ok(mut current) = state.active.lock() else {
                    break;
                };
                let Some(a) = current.as_ref().filter(|a| a.id == id).cloned() else {
                    break;
                };
                let delay = match a.session.try_lock() {
                    Ok(s) => s.deadline.saturating_duration_since(Instant::now()),
                    Err(_) => Duration::from_millis(250),
                };
                if delay.is_zero() {
                    a.cancel.store(true, Ordering::Relaxed);
                    *current = None;
                    break;
                }
                delay
            };
            tokio::time::sleep(wait).await;
        }
    });
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
pub(crate) async fn begin_mi_login(region: String, app: tauri::AppHandle) -> Result<LoginStart> {
    tauri::async_runtime::spawn_blocking(move || {
        let cancel = Arc::new(AtomicBool::new(false));
        let session = Session::new(&region, cancel.clone())?;
        let active = Active {
            id: hex::encode(rand::random::<[u8; 16]>()),
            session: Arc::new(Mutex::new(session)),
            cancel,
        };
        {
            let state = app.state::<CloudState>();
            let mut s = state.active.lock().map_err(|_| "米家会话不可用")?;
            if let Some(old) = s.replace(active.clone()) {
                old.cancel.store(true, Ordering::Relaxed);
            }
        }
        let result = (|| {
            let mut s = active.session.lock().map_err(|_| "米家会话不可用")?;
            let qr = s.start()?;
            Ok(LoginStart {
                session_id: active.id.clone(),
                qr_image_data_url: qr,
                expires_at: s.expires_at,
            })
        })();
        if result.is_err() {
            let _ = app.state::<CloudState>().cancel(&active.id);
        } else {
            expire_session(app.clone(), active.id.clone());
        }
        result
    })
    .await
    .map_err(|_| "米家登录任务失败".to_owned())?
}
#[cfg_attr(not(feature = "lifecycle-validation"), tauri::command)]
pub(crate) async fn poll_mi_login(
    session_id: String,
    app: tauri::AppHandle,
) -> Result<LoginStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        let a = app.state::<CloudState>().get(&session_id)?;
        let result = a.session.lock().map_err(|_| "米家会话不可用")?.poll();
        result
    })
    .await
    .map_err(|_| "米家登录任务失败".to_owned())?
}
#[tauri::command]
pub(crate) async fn list_mi_devices(
    session_id: String,
    app: tauri::AppHandle,
) -> Result<DeviceList> {
    tauri::async_runtime::spawn_blocking(move || {
        let a = app.state::<CloudState>().get(&session_id)?;
        let result = a.session.lock().map_err(|_| "米家会话不可用")?.list();
        result
    })
    .await
    .map_err(|_| "米家设备查询任务失败".to_owned())?
}
#[tauri::command]
pub(crate) async fn import_mi_lamp(
    session_id: String,
    device_id: String,
    app: tauri::AppHandle,
) -> Result<crate::LampConfig> {
    tauri::async_runtime::spawn_blocking(move || {
        let a = app.state::<CloudState>().get(&session_id)?;
        let result = {
            let s = a.session.lock().map_err(|_| "米家会话不可用")?;
            s.check()?;
            let d = s
                .devices
                .get(&device_id)
                .filter(|d| d.summary.eligible)
                .ok_or("请选择已读取且可导入的挂灯设备")?;
            crate::save_lamp_config(
                d.summary.ip.clone().ok_or("设备缺少局域网IP")?,
                d.token.to_string(),
                app.state::<crate::Backend>(),
            )
        };
        if result.is_ok() {
            app.state::<CloudState>().cancel(&session_id)?;
        }
        result
    })
    .await
    .map_err(|_| "米家设备导入任务失败".to_owned())?
}
#[tauri::command]
pub(crate) fn cancel_mi_login(session_id: String, state: State<CloudState>) -> Result<()> {
    state.cancel(&session_id)
}

#[cfg(feature = "lifecycle-validation")]
pub(crate) mod lifecycle_mock {
    use super::{Active, CloudState, LoginStart, LoginStatus, Result, Session};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    use tauri::Manager;
    static FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();
    pub(crate) fn cancelled() -> bool {
        FLAG.get().is_some_and(|v| v.load(Ordering::Relaxed))
    }
    #[tauri::command]
    pub(crate) fn begin_mi_login(region: String, app: tauri::AppHandle) -> Result<LoginStart> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut session = Session::new(&region, cancel.clone())?;
        session.deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        session.expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 600000;
        let _ = FLAG.set(cancel.clone());
        let expires_at = session.expires_at;
        *app.state::<CloudState>()
            .active
            .lock()
            .map_err(|_| "mock lock")? = Some(Active {
            id: "LIFECYCLE_VALIDATION_ONLY".into(),
            session: Arc::new(Mutex::new(session)),
            cancel,
        });
        Ok(LoginStart {session_id: "LIFECYCLE_VALIDATION_ONLY".into(), expires_at,
            qr_image_data_url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jR1cAAAAASUVORK5CYII=".into()})
    }
    #[tauri::command]
    pub(crate) fn poll_mi_login(session_id: String, app: tauri::AppHandle) -> Result<LoginStatus> {
        let active = app.state::<CloudState>().get(&session_id)?;
        let session = active.session.lock().map_err(|_| "mock lock")?;
        Ok(LoginStatus {
            status: "pending",
            expires_at: session.expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trusted_hosts_only() {
        for u in [
            "https://account.xiaomi.com/qr",
            "https://sts.api.io.mi.com/sts",
        ] {
            assert!(trusted(u).is_ok());
        }
        for u in [
            "http://mi.com/",
            "https://mi.com.evil.test/",
            "https://evilmi.com/",
            "https://user@mi.com/",
            "https://mi.com:444/",
        ] {
            assert!(trusted(u).is_err());
        }
    }
    #[test]
    fn parses_prefix_and_session_errors() {
        assert_eq!(
            parse_json(br#"&&&START&&&{"code":0,"result":[]}"#).unwrap()["code"],
            0
        );
        assert!(api_result(json!({"code":-10001,"message":"secret"}))
            .unwrap_err()
            .find("secret")
            .is_none());
        assert!(parse_json(b"invalid").is_err());
    }
    #[test]
    fn listing_never_serializes_tokens() {
        let d=parse_device(&json!({"did":123,"name":"display lamp","model":"yeelink.light.lamp22","localip":"192.168.1.7","token":"0123456789abcdef0123456789abcdef"})).unwrap().unwrap();
        assert!(d.summary.eligible);
        let s = serde_json::to_string(&d.summary).unwrap();
        assert!(!s.contains("token"));
        assert!(!s.contains(d.token.as_str()));
    }
    #[test]
    fn device_gates() {
        for (model, ip, token) in [
            (
                "other.model",
                "192.168.1.7",
                "0123456789abcdef0123456789abcdef",
            ),
            (
                "yeelink.light.lamp22",
                "8.8.8.8",
                "0123456789abcdef0123456789abcdef",
            ),
            (
                "yeelink.light.lamp22",
                "192.168.1.7",
                "00000000000000000000000000000000",
            ),
        ] {
            let d = parse_device(&json!({"did":"123","model":model,"localip":ip,"token":token}))
                .unwrap()
                .unwrap();
            assert!(!d.summary.eligible);
        }
        assert!(parse_device(&json!({"name":"no id"})).unwrap().is_none());
    }
    #[test]
    fn expiry_cancel_and_invalid_region_no_network() {
        assert!(Session::new("bad", Arc::new(AtomicBool::new(false))).is_err());
        let flag = Arc::new(AtomicBool::new(false));
        let mut s = Session::new("cn", flag.clone()).unwrap();
        assert!(s.check().unwrap_err().contains("过期"));
        s.deadline = Instant::now() + Duration::from_secs(5);
        assert!(s.check().is_ok());
        flag.store(true, Ordering::Relaxed);
        assert!(s.start().unwrap_err().contains("取消"));
    }
    #[test]
    fn unknown_session_is_rejected() {
        let s = CloudState::default();
        assert!(s.get("untrusted").is_err());
        assert!(s.cancel("gone").is_ok());
    }
    #[test]
    fn window_close_revokes_pending_and_authenticated_sessions() {
        for authenticated in [false, true] {
            let flag = Arc::new(AtomicBool::new(false));
            let mut session = Session::new("cn", flag.clone()).unwrap();
            session.deadline = Instant::now() + Duration::from_secs(30);
            session.authenticated = authenticated;
            let shared = Arc::new(Mutex::new(session));
            let state = CloudState {
                active: Mutex::new(Some(Active {
                    id: "window-session".into(),
                    session: shared.clone(),
                    cancel: flag.clone(),
                })),
            };
            // A blocked HTTP operation may hold this lock. Revocation must not need it.
            let held = shared.lock().unwrap();
            state.cancel_all().unwrap();
            assert!(flag.load(Ordering::Relaxed));
            assert!(state.get("window-session").is_err());
            assert!(held.check().unwrap_err().contains("取消"));
            state.cancel_all().unwrap();
        }
    }
    #[test]
    fn device_list_aliases_and_invalid_schema_are_explicit() {
        for response in [
            json!({"device_info":[]}),
            json!({"list":[]}),
            json!({"device_info":null,"list":[]}),
        ] {
            assert!(device_rows(&response).unwrap().is_empty());
        }
        for response in [
            json!({}),
            json!({"device_info":null}),
            json!({"list":{}}),
            json!({"token":"private-value","123-private-account":"private"}),
        ] {
            let error = device_rows(&response).unwrap_err();
            assert!(error.contains("结构不兼容"));
            assert!(!error.contains("private"));
            assert!(!error.contains("token"));
        }
        assert!(validate_homes(&json!({"homelist":[],"share_home_list":null})).is_ok());
        assert!(validate_homes(&json!({"homelist":{}})).is_err());
        assert!(validate_homes(&json!({})).is_err());
        assert_eq!(has_more(&json!({"has_more":1})).unwrap(), true);
        assert_eq!(has_more(&json!({"has_more":false})).unwrap(), false);
        assert!(has_more(&json!({"has_more":"secret"})).is_err());
    }
    #[test]
    fn fallback_is_once_and_warns_instead_of_hiding_primary_error() {
        let calls = std::cell::Cell::new(0);
        let (devices,warnings)=recover_listing(Err("device_info:null".into()),||{calls.set(calls.get()+1);Ok(json!({"list":[{"did":"synthetic-device","name":"lamp","model":"yeelink.light.lamp22","localip":"192.168.1.7","token":"0123456789abcdef0123456789abcdef"}],"has_more":1}))}).unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(devices.len(), 1);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("device_info:null"));
        let result = recover_listing(Err("primary failed".into()), || {
            Ok(json!({"unexpected":"private"}))
        });
        let error = match result {
            Err(e) => e,
            Ok(_) => panic!("Malformed fallback must fail"),
        };
        assert!(error.contains("primary failed"));
        assert!(error.contains("兼容设备读取也失败"));
        assert!(!error.contains("private"));
        let (_, warnings) = recover_listing(Ok(HashMap::new()), || {
            panic!("Must not request fallback on valid empty primary list")
        })
        .unwrap();
        assert!(warnings.is_empty());
        let (devices, warnings) =
            recover_listing(Err("primary failed".into()), || Ok(json!({"list":[]}))).unwrap();
        assert!(devices.is_empty());
        assert!(!warnings.is_empty());
    }
    #[test]
    #[ignore = "Explicit anonymous Xiaomi QR network smoke test; never scans or logs in"]
    fn anonymous_qr_begin_cancel_network_smoke() -> Result<()> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut session = Session::new("cn", cancel.clone())?;
        let result = session.start();
        // Cancel before inspecting or reporting the result. No poll/login/API calls follow.
        cancel.store(true, Ordering::Relaxed);
        let mut qr = result?;
        let mut png = BASE64
            .decode(
                qr.strip_prefix("data:image/png;base64,")
                    .ok_or("Unexpected image format")?,
            )
            .map_err(|_| "Invalid PNG encoding")?;
        let dims = if png.len() >= 24 {
            Some((
                u32::from_be_bytes(png[16..20].try_into().unwrap()),
                u32::from_be_bytes(png[20..24].try_into().unwrap()),
            ))
        } else {
            None
        };
        let qr_host = trusted(session.qr["qr"].as_str().ok_or("QR URL missing")?)?
            .host_str()
            .unwrap_or("")
            .to_owned();
        let poll_host = trusted(session.qr["lp"].as_str().ok_or("Poll URL missing")?)?
            .host_str()
            .unwrap_or("")
            .to_owned();
        assert!(
            !session.authenticated
                && session.security.is_empty()
                && session.service_token.is_empty()
                && session.devices.is_empty()
        );
        assert!(session.check().unwrap_err().contains("取消"));
        println!("anonymous_qr: PNG bytes={}, dimensions={dims:?}, image_host={qr_host}, poll_host={poll_host}; locally cancelled; authenticated=false; account_data_requests=0",png.len());
        qr.zeroize();
        png.zeroize();
        let state = CloudState {
            active: Mutex::new(Some(Active {
                id: "anonymous-smoke".into(),
                session: Arc::new(Mutex::new(session)),
                cancel,
            })),
        };
        state.cancel("anonymous-smoke")?;
        assert!(state
            .active
            .lock()
            .map_err(|_| "Smoke session lock failed")?
            .is_none());
        Ok(())
    }
}
