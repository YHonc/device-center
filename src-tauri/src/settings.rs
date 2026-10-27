//! Typed, bounded settings operations; no hardware writes occur until apply_sync.
use serde::{Deserialize, Serialize};
pub(crate) mod atk_extensions;
pub(crate) mod kx_extensions;
pub(crate) mod kx_mapping;
pub(crate) mod mappings;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub(crate) struct SettingsError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}
fn err(code: &'static str, message: impl Into<String>) -> SettingsError {
    SettingsError {
        code,
        message: message.into(),
    }
}
fn kx_stage(mut error: SettingsError, stage: &str) -> SettingsError {
    error.message = format!("{stage}：{}", error.message);
    error
}
type Result<T> = std::result::Result<T, SettingsError>;
#[cfg(feature = "lifecycle-validation")]
pub(crate) fn lifecycle_fixture(id: &str) -> SettingsSnapshot {
    snapshot(
        id,
        "LIFECYCLE_VALIDATION_ONLY",
        Some([1, 0, 4, 0, 1, 0, 0, 0, 0, 0]),
    )
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSnapshot {
    device_id: String,
    model: String,
    fields: Vec<Field>,
    sampled_at: u64,
    verified_on_hardware: bool,
    state_token: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Field {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    value: Option<u32>,
    available: bool,
    reason: Option<&'static str>,
    min: Option<u32>,
    max: Option<u32>,
    step: Option<u32>,
    options: Option<Vec<Choice>>,
}
#[derive(Serialize)]
struct Choice {
    label: String,
    value: u32,
}
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SettingsChange {
    report_rate_hz: Option<u32>,
    dpi_level: Option<u32>,
    lighting_brightness: Option<u32>,
    sleep_seconds: Option<u32>,
    low_latency: Option<bool>,
    mac_mode: Option<bool>,
    win_lock: Option<bool>,
    expected_report_rate_hz: Option<u32>,
    expected_dpi_level: Option<u32>,
    expected_state: Option<String>,
}
fn field(
    key: &'static str,
    label: &'static str,
    value: Option<u32>,
    options: Option<Vec<Choice>>,
    reason: Option<&'static str>,
) -> Field {
    Field {
        key,
        label,
        kind: if options.is_some() {
            "select"
        } else {
            "number"
        },
        value,
        available: reason.is_none(),
        reason,
        min: None,
        max: None,
        step: None,
        options,
    }
}
fn snapshot(id: &str, model: &str, block: Option<[u8; 10]>) -> SettingsSnapshot {
    let unavailable = Some("尚未形成准确型号的安全读写与读回契约");
    let mut rate = field("reportRateHz", "回报率", None, None, unavailable);
    let mut dpi = field("dpiLevel", "DPI 档位", None, None, unavailable);
    if let Some(b) = block {
        rate = field(
            "reportRateHz",
            "回报率",
            hz(b[0]),
            Some(
                [125, 250, 500, 1000]
                    .into_iter()
                    .map(|v| Choice {
                        label: format!("{v} Hz"),
                        value: v,
                    })
                    .collect(),
            ),
            None,
        );
        dpi = field(
            "dpiLevel",
            "DPI 档位",
            Some(b[4] as u32),
            Some(
                (0..b[2] as u32)
                    .map(|v| Choice {
                        label: format!("档位 {}", v + 1),
                        value: v,
                    })
                    .collect(),
            ),
            None,
        );
    }
    SettingsSnapshot {
        device_id: id.into(),
        model: model.into(),
        fields: vec![
            rate,
            dpi,
            field("lightingBrightness", "灯光亮度", None, None, unavailable),
            field("sleepSeconds", "休眠时间（秒）", None, None, unavailable),
        ],
        sampled_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        verified_on_hardware: false,
        state_token: block.map(hex::encode),
    }
}
fn hz(raw: u8) -> Option<u32> {
    match raw {
        8 => Some(125),
        4 => Some(250),
        2 => Some(500),
        1 => Some(1000),
        _ => None,
    }
}
fn raw_hz(value: u32) -> Result<u8> {
    match value {
        125 => Ok(8),
        250 => Ok(4),
        500 => Ok(2),
        1000 => Ok(1),
        _ => Err(err(
            "invalid_parameter",
            "当前接收器仅支持125/250/500/1000 Hz",
        )),
    }
}
fn request(command: u8, block: Option<[u8; 10]>) -> [u8; 17] {
    let mut r = [0u8; 17];
    r[0] = 8;
    r[1] = command;
    r[5] = 10;
    if let Some(b) = block {
        r[6..16].copy_from_slice(&b);
    }
    r[16] = 0x4du8.wrapping_sub(r[1..16].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    r
}
fn parse(response: &[u8]) -> Result<[u8; 10]> {
    if response.len() != 17
        || response[0] != 8
        || response[1] != 8
        || response[3..5] != [0, 0]
        || response[5] != 10
    {
        return Err(err(
            "invalid_response",
            "设置读取回复长度、报告ID、命令或EEPROM地址不匹配",
        ));
    }
    // The official driver does not define rejection codes. Fail closed on nonzero status.
    if response[2] != 0 {
        return Err(err(
            "device_rejected",
            format!("设备设置状态非零：{}", response[2]),
        ));
    }
    let b: [u8; 10] = response[6..16].try_into().unwrap();
    if hz(b[0]).is_none()
        || !(1..=8).contains(&b[2])
        || b[4] >= b[2]
        || [0, 2, 4]
            .into_iter()
            .any(|i| b[i].wrapping_add(b[i + 1]) != 85)
    {
        return Err(err("invalid_response", "回报率、DPI档位或伴随校验字段无效"));
    }
    Ok(b)
}
trait Transport {
    fn send(&mut self, packet: &[u8]) -> Result<()>;
    fn receive(&mut self) -> Result<Vec<u8>>;
    fn receive_for(&mut self, _timeout: Duration) -> Result<Vec<u8>> {
        self.receive()
    }
}
struct HidTransport(hidapi::HidDevice);
impl Transport for HidTransport {
    fn send(&mut self, p: &[u8]) -> Result<()> {
        let n = self
            .0
            .write(p)
            .map_err(|e| err("connection", e.to_string()))?;
        if n != p.len() {
            return Err(err("connection", "ATK设置Output请求发送不完整"));
        }
        Ok(())
    }
    fn receive(&mut self) -> Result<Vec<u8>> {
        let mut r = [0u8; 64];
        let n = self
            .0
            .read_timeout(&mut r, 200)
            .map_err(|e| err("connection", e.to_string()))?;
        if n == 0 {
            return Err(err("timeout", "设备可能休眠，请移动鼠标后重新读取"));
        }
        Ok(r[..n].to_vec())
    }
}
struct KxTransport {
    device: hidapi::HidDevice,
}
impl Transport for KxTransport {
    fn send(&mut self, p: &[u8]) -> Result<()> {
        let n = self
            .device
            .write(p)
            .map_err(|e| err("connection", e.to_string()))?;
        if n != p.len() {
            return Err(err("connection", "KX75请求发送不完整"));
        }
        Ok(())
    }
    fn receive(&mut self) -> Result<Vec<u8>> {
        self.receive_for(Duration::from_millis(500))
    }
    fn receive_for(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let mut r = [0u8; 64];
        let n = self
            .device
            .read_timeout(&mut r, timeout.as_millis().clamp(1, 10_000) as i32)
            .map_err(|e| err("connection", e.to_string()))?;
        if n == 0 {
            return Err(err("timeout", "KX75响应超时"));
        }
        Ok(r[..n].to_vec())
    }
}
fn kx_receive(t: &mut impl Transport, cmd: u8, seq: u8) -> Result<Vec<u8>> {
    kx_receive_until(t,cmd,seq,Instant::now()+Duration::from_millis(500))
}
fn kx_receive_until(t:&mut impl Transport,cmd:u8,seq:u8,deadline:Instant)->Result<Vec<u8>> {
    for _ in 0..32 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let p = t.receive_for(left)?;
        if p.len() < 4 || p[0] != 0x13 || p[1] != cmd || p[3] != seq {
            continue;
        }
        kx_check(&p, cmd)?;
        return Ok(p);
    }
    Err(err("timeout", "KX75未收到匹配命令及序号的响应"))
}
fn kx_packet(cmd: u8, total: u8, seq: u8, data: &[u8]) -> [u8; 20] {
    let mut p = [0u8; 20];
    p[0] = 0x13;
    p[1] = cmd;
    p[2] = total;
    p[3] = seq;
    p[4] = data.len() as u8;
    p[5..5 + data.len()].copy_from_slice(data);
    p[19] = p[..19].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    p
}
fn kx_check(p: &[u8], cmd: u8) -> Result<()> {
    if p.len() != 20
        || p[0] != 0x13
        || p[1] != cmd
        || p[..19].iter().fold(0u8, |a, b| a.wrapping_add(*b)) != p[19]
    {
        return Err(err(
            "invalid_response",
            "KX75设置回复长度、命令或校验不匹配",
        ));
    }
    Ok(())
}
fn kx_read(t: &mut impl Transport) -> Result<[u8; 128]> {
    t.send(&kx_packet(0x44, 1, 0, &[]))?;
    let mut data = Vec::with_capacity(140);
    for seq in 0..10 {
        let p = kx_receive(t, 0x44, seq)?;
        if p[2] != 10 || p[3] != seq {
            return Err(err("invalid_response", "KX75性能块分包数量或序号不匹配"));
        }
        data.extend_from_slice(&p[5..19]);
    }
    let b: [u8; 128] = data[..128].try_into().unwrap();
    if b[24] > 40 {
        return Err(err("invalid_response", "KX75休眠现值超出已核验范围"));
    }
    Ok(b)
}
fn kx_apply(t: &mut impl Transport, c: SettingsChange) -> Result<[u8; 128]> {
    if c.dpi_level.is_some() || c.lighting_brightness.is_some() {
        return Err(err("unsupported", "KX75基础性能不支持DPI或独立灯光亮度"));
    }
    if c.sleep_seconds.is_none() && c.report_rate_hz.is_none() && c.low_latency.is_none()
        && c.mac_mode.is_none() && c.win_lock.is_none() {
        return Err(err("invalid_parameter", "请选择性能设置"));
    }
    if c.sleep_seconds.is_some_and(|seconds| seconds > 1200 || seconds % 30 != 0) {
        return Err(err(
            "invalid_parameter",
            "休眠范围0至1200秒，步长30秒；0为永不休眠",
        ));
    }
    let rate = match c.report_rate_hz {
        None => None,
        Some(250) => Some(1),
        Some(500) => Some(2),
        Some(1000) => Some(3),
        Some(_) => return Err(err("invalid_parameter", "KX75回报率仅支持250/500/1000 Hz")),
    };
    let before = kx_read(t).map_err(|e|kx_stage(e,"应用前读取性能设置失败（尚未发送SET）"))?;
    if c.expected_state.as_deref() != Some(hex::encode(before).as_str()) {
        return Err(err(
            "stale_state",
            "KX75完整性能设置已变化或缺少预览快照，请刷新",
        ));
    }
    let mut wanted = before;
    if (c.report_rate_hz.is_some() && !(1..=3).contains(&before[1]))
        || (c.low_latency.is_some() && ![0,2].contains(&before[3]))
        || (c.mac_mode.is_some() && ![0,2].contains(&before[27]))
        || (c.mac_mode == Some(true) && ![0,1].contains(&before[15]))
        || (c.win_lock.is_some() && (! [0,1].contains(&before[15]) || ![0,2].contains(&before[27]))) {
        return Err(err("unsupported", "目标性能字段现值未知，请保留原值并刷新"));
    }
    if let Some(seconds) = c.sleep_seconds { wanted[24] = (seconds / 30) as u8; }
    if let Some(rate) = rate { wanted[1] = rate; }
    if let Some(enabled) = c.low_latency { wanted[3] = if enabled { 0 } else { 2 }; }
    if let Some(enabled) = c.mac_mode { wanted[27] = if enabled { 2 } else { 0 }; }
    if wanted[27] == 2 {
        if c.win_lock == Some(true) {
            return Err(err("invalid_parameter", "Mac模式下不能启用Win锁"));
        }
        if c.mac_mode == Some(true) { wanted[15] = 0; }
    }
    if let Some(enabled) = c.win_lock { wanted[15] = u8::from(enabled); }
    if wanted == before {
        return Ok(before);
    }
    for (seq, chunk) in wanted.chunks(14).enumerate() {
        t.send(&kx_packet(4, 10, seq as u8, chunk)).map_err(|e|kx_stage(e,"性能SET发送失败，可能部分写入，请刷新"))?;
        let ack = kx_receive(t, 4, seq as u8).map_err(|e|kx_stage(e,"性能SET已发送，等待ACK失败，请刷新"))?;
        if ack[3] != seq as u8 {
            return Err(err(
                "invalid_response",
                "KX75写入ACK序号不匹配；可能已部分写入，请刷新",
            ));
        }
    }
    for _ in 0..2 {
        if kx_read(t).map_err(|e|kx_stage(e,"性能SET已发送，完整读回失败，请刷新"))? == wanted {
            return Ok(wanted);
        }
    }
    Err(err(
        "readback_mismatch",
        "KX75完整性能块读回不一致，可能已部分写入，请刷新",
    ))
}
fn kx_snapshot(id: &str, b: [u8; 128]) -> SettingsSnapshot {
    let mut s = snapshot(id, "MCHOSE KX75", None);
    s.state_token = Some(hex::encode(b));
    s.fields[3] = field(
        "sleepSeconds",
        "休眠时间（秒，0为永不）",
        Some(b[24] as u32 * 30),
        None,
        None,
    );
    s.fields[3].min = Some(0);
    s.fields[3].max = Some(1200);
    s.fields[3].step = Some(30);
    s.fields[0] = field("reportRateHz", "回报率", match b[1] {1=>Some(250),2=>Some(500),3=>Some(1000),_=>None},
        Some([250,500,1000].into_iter().map(|value|Choice{label:format!("{value} Hz"),value}).collect()), if (1..=3).contains(&b[1]) {None} else {Some("回报率现值未知，保留原值")});
    for (key,label,value,reason) in [
        ("lowLatency","TopSpeed极速模式",match b[3] {0=>Some(1),2=>Some(0),_=>None},None),
        ("macMode","Mac模式",match b[27] {2=>Some(1),0=>Some(0),_=>None},None),
        ("winLock","Win键锁定",match b[15] {1=>Some(1),0=>Some(0),_=>None},if b[27]==2 {Some("Mac模式会自动关闭Win锁")} else if b[27]!=0 {Some("Mac模式现值未知，保留Win锁原值")} else {None}),
    ] {
        s.fields.push(field(key,label,value,Some(vec![Choice{label:"关闭".into(),value:0},Choice{label:"开启".into(),value:1}]),reason.or(if value.is_none() {Some("当前字段编码未知，保留原值")} else {None})));
    }
    s
}
fn read(t: &mut impl Transport) -> Result<[u8; 10]> {
    t.send(&request(8, None))?;
    // Discard only unrelated/previous write ACKs, with a strict finite budget.
    for _ in 0..4 {
        let r = t.receive()?;
        if r.len() > 1 && r[0] == 8 && r[1] == 8 {
            return parse(&r);
        }
    }
    Err(err("invalid_response", "未收到匹配的设置读取回复"))
}
fn apply(t: &mut impl Transport, c: SettingsChange) -> Result<[u8; 10]> {
    if c.lighting_brightness.is_some() || c.sleep_seconds.is_some() || c.low_latency.is_some()
        || c.mac_mode.is_some() || c.win_lock.is_some() {
        return Err(err("unsupported", "该设置尚未支持安全写入"));
    }
    if c.report_rate_hz.is_none() && c.dpi_level.is_none() {
        return Err(err("invalid_parameter", "请选择至少一项变更"));
    }
    if let Some(v) = c.report_rate_hz {
        raw_hz(v)?;
    }
    let before = read(t)?;
    if c.expected_state
        .as_deref()
        .map(|s| s != hex::encode(before))
        .unwrap_or(
            c.expected_report_rate_hz != hz(before[0])
                || c.expected_dpi_level != Some(before[4] as u32),
        )
    {
        return Err(err(
            "stale_state",
            "设备现值已变化或缺少预览快照，请刷新后重新预览",
        ));
    }
    let mut wanted = before;
    if let Some(v) = c.report_rate_hz {
        wanted[0] = raw_hz(v)?;
        wanted[1] = 85u8.wrapping_sub(wanted[0]);
    }
    if let Some(v) = c.dpi_level {
        if v >= before[2] as u32 {
            return Err(err("invalid_parameter", "DPI档位超出设备已有档位范围"));
        }
        wanted[4] = v as u8;
        wanted[5] = 85 - v as u8;
    }
    if wanted == before {
        return Ok(before);
    }
    t.send(&request(7, Some(wanted)))?;
    for _ in 0..3 {
        if read(t)? == wanted {
            return Ok(wanted);
        }
    }
    Err(err(
        "readback_mismatch",
        "读回与请求不一致；设备可能已经部分写入，请刷新确认",
    ))
}
fn atk(d: &hidapi::DeviceInfo) -> bool {
    d.vendor_id() == 0x373b
        && d.product_id() == 0x101a
        && d.usage_page() == 0xff02
        && d.usage() == 2
        && d.interface_number() == 1
}
enum DeviceTransport {
    Atk(HidTransport),
    Kx(KxTransport),
}
fn open(id: &str) -> Result<DeviceTransport> {
    let api = hidapi::HidApi::new().map_err(|e| err("connection", e.to_string()))?;
    let d = api
        .device_list()
        .find(|d| hex::encode(d.path().to_bytes()) == id)
        .ok_or_else(|| err("disconnected", "设备不存在，请重新枚举"))?;
    if atk(d) {
        return Ok(DeviceTransport::Atk(HidTransport(
            d.open_device(&api)
                .map_err(|e| err("connection", e.to_string()))?,
        )));
    }
    if d.vendor_id() == 0x3554
        && d.product_id() == 0xfa09
        && d.usage_page() == 0xff02
        && d.usage() == 2
        && d.interface_number() == 1
        && matches!(
            d.product_string(),
            Some(
                "2.4G Wireless Receiver"
                    | "2.4G Wireless Receiver-KX75"
                    | "MCHOSE 2.4G Wireless-KX75"
            )
        )
    {
        let device = d
            .open_device(&api)
            .map_err(|e| err("connection", e.to_string()))?;
        crate::verify_kx75(&device).map_err(|e| err("unsupported", e))?;
        return Ok(DeviceTransport::Kx(KxTransport { device }));
    }
    Err(err("unsupported", "设备接口不在设置协议白名单"))
}
pub(crate) fn read_sync(device_id: &str) -> Result<SettingsSnapshot> {
    let queue = crate::device_queue(device_id);
    let _guard = queue.lock().map_err(|_| err("queue", "HID队列不可用"))?;
    match open(device_id)? {
        DeviceTransport::Atk(mut t) => Ok(snapshot(
            device_id,
            "ATK Mouse 1K Dongle",
            Some(read(&mut t)?),
        )),
        DeviceTransport::Kx(mut t) => Ok(kx_snapshot(device_id, kx_read(&mut t).map_err(|e|kx_stage(e,"读取性能设置失败（未发送设置写入）"))?)),
    }
}
/// Explicit diagnostic only: never permits SET or changes the production write whitelist.
#[cfg(test)]
pub(crate) fn diagnostic_read_sync(device_id: &str) -> Result<SettingsSnapshot> {
    let queue = crate::device_queue(device_id);
    let _guard = queue.lock().map_err(|_| err("queue", "HID队列不可用"))?;
    let api = hidapi::HidApi::new().map_err(|e| err("connection", e.to_string()))?;
    let d = api
        .device_list()
        .find(|d| {
            hex::encode(d.path().to_bytes()) == device_id
                && d.vendor_id() == 0x373b
                && d.product_id() == 0x101a
                && d.usage_page() == 0xff02
                && d.usage() == 2
                && d.interface_number() == 1
        })
        .ok_or_else(|| {
            err(
                "unsupported",
                "只读诊断仅允许已枚举ATK FF02/usage2/interface1",
            )
        })?;
    let mut t = HidTransport(
        d.open_device(&api)
            .map_err(|e| err("connection", e.to_string()))?,
    );
    Ok(snapshot(
        device_id,
        "ATK Mouse 1K Dongle",
        Some(read(&mut t)?),
    ))
}
pub(crate) fn apply_sync(device_id: &str, change: SettingsChange) -> Result<SettingsSnapshot> {
    let queue = crate::device_queue(device_id);
    let _guard = queue.lock().map_err(|_| err("queue", "HID队列不可用"))?;
    match open(device_id)? {
        DeviceTransport::Atk(mut t) => Ok(snapshot(
            device_id,
            "ATK Mouse 1K Dongle",
            Some(apply(&mut t, change)?),
        )),
        DeviceTransport::Kx(mut t) => Ok(kx_snapshot(device_id, kx_apply(&mut t, change)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    #[test]
    fn hardware_read_block_fields() {
        // Sanitized EEPROM block observed by the bounded, read-only FF02 diagnostic.
        let b = [1, 84, 4, 81, 1, 84, 0, 85, 0, 85];
        assert_eq!(parse(&reply(b)).unwrap(), b);
        assert_eq!(hz(b[0]), Some(1000));
        assert_eq!(b[2], 4);
        assert_eq!(b[4], 1);
    }
    struct Mock {
        replies: VecDeque<Vec<u8>>,
        sent: Vec<Vec<u8>>,
    }
    impl Transport for Mock {
        fn send(&mut self, p: &[u8]) -> Result<()> {
            self.sent.push(p.to_vec());
            Ok(())
        }
        fn receive(&mut self) -> Result<Vec<u8>> {
            self.replies
                .pop_front()
                .ok_or_else(|| err("timeout", "mock timeout"))
        }
    }
    fn block() -> [u8; 10] {
        [1, 84, 3, 82, 0, 85, 1, 84, 2, 83]
    }
    fn reply(b: [u8; 10]) -> Vec<u8> {
        let mut r = request(8, Some(b));
        r[2] = 0;
        r.to_vec()
    }
    fn change() -> SettingsChange {
        SettingsChange {
            report_rate_hz: Some(500),
            expected_report_rate_hz: Some(1000),
            expected_dpi_level: Some(0),
            ..Default::default()
        }
    }
    #[test]
    fn exact_get_packet() {
        assert_eq!(
            request(8, None),
            [8, 8, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 59]
        );
    }
    #[test]
    fn write_preserves_and_verifies() {
        let mut after = block();
        after[0] = 2;
        after[1] = 83;
        let mut m = Mock {
            replies: vec![reply(block()), vec![8, 7], reply(after)].into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, change()).unwrap(), after);
        assert_eq!(m.sent[1], request(7, Some(after)));
        assert_eq!(
            m.sent[1][1..].iter().fold(0u8, |a, b| a.wrapping_add(*b)),
            77
        );
    }
    #[test]
    fn stale_never_writes() {
        let mut m = Mock {
            replies: vec![reply(block())].into(),
            sent: vec![],
        };
        let mut c = change();
        c.expected_dpi_level = Some(1);
        assert_eq!(apply(&mut m, c).unwrap_err().code, "stale_state");
        assert_eq!(m.sent.len(), 1);
    }
    #[test]
    fn mismatch_is_failure() {
        let mut m = Mock {
            replies: vec![reply(block()); 4].into(),
            sent: vec![],
        };
        assert_eq!(
            apply(&mut m, change()).unwrap_err().code,
            "readback_mismatch"
        );
        assert_eq!(m.sent.len(), 5);
    }
    #[test]
    fn invalid_and_timeout() {
        let mut r = reply(block());
        r[5] = 9;
        assert_eq!(parse(&r).unwrap_err().code, "invalid_response");
        let mut m = Mock {
            replies: VecDeque::new(),
            sent: vec![],
        };
        assert_eq!(read(&mut m).unwrap_err().code, "timeout");
        let mut c = change();
        c.report_rate_hz = Some(8000);
        assert_eq!(apply(&mut m, c).unwrap_err().code, "invalid_parameter");
        assert_eq!(m.sent.len(), 1);
    }
    #[test]
    fn dpi_range_and_unknown_fields() {
        let mut m = Mock {
            replies: vec![reply(block())].into(),
            sent: vec![],
        };
        let mut c = change();
        c.dpi_level = Some(3);
        assert_eq!(apply(&mut m, c).unwrap_err().code, "invalid_parameter");
        assert_eq!(m.sent.len(), 1);
        assert!(serde_json::from_str::<SettingsChange>(r#"{"rawCommand":7}"#).is_err());
    }
    fn kx_replies(b: [u8; 128]) -> Vec<Vec<u8>> {
        b.chunks(14)
            .enumerate()
            .map(|(i, c)| kx_packet(0x44, 10, i as u8, c).to_vec())
            .collect()
    }
    fn kx_change(b: [u8; 128]) -> SettingsChange {
        SettingsChange {
            sleep_seconds: Some(60),
            expected_state: Some(hex::encode(b)),
            ..Default::default()
        }
    }
    #[test]
    fn kx_get_packet() {
        let p = kx_packet(0x44, 1, 0, &[]);
        assert_eq!(
            p,
            [0x13, 0x44, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x58]
        );
    }
    #[test]
    fn kx_full_block_write_and_verify() {
        let before = [0u8; 128];
        let mut after = before;
        after[24] = 2;
        let mut replies = kx_replies(before);
        replies.extend((0..10).map(|i| kx_packet(4, 10, i, &[]).to_vec()));
        replies.extend(kx_replies(after));
        let mut m = Mock {
            replies: replies.into(),
            sent: vec![],
        };
        assert_eq!(kx_apply(&mut m, kx_change(before)).unwrap(), after);
        assert_eq!(m.sent.len(), 12);
        assert_eq!(m.sent[10][4], 2);
        assert_eq!(m.sent[2][15], 2);
    }
    #[test]
    fn kx_bad_sequence_and_checksum() {
        let mut replies = kx_replies([0; 128]);
        replies[1] = kx_packet(0x44, 10, 3, &[0; 14]).to_vec();
        let mut m = Mock {
            replies: replies.into(),
            sent: vec![],
        };
        assert_eq!(kx_read(&mut m).unwrap_err().code, "timeout");
        let mut p = kx_packet(0x44, 10, 0, &[0; 14]);
        p[19] ^= 1;
        assert!(kx_check(&p, 0x44).is_err());
    }
    #[test]
    fn kx_skips_stale_reports_with_bounded_budget() {
        let b = [0; 128];
        let mut replies = vec![
            vec![0, 1, 2],
            kx_packet(0x4a, 1, 0, &[0; 14]).to_vec(),
            kx_packet(0x44, 10, 7, &[0; 14]).to_vec(),
        ];
        replies.extend(kx_replies(b));
        let mut m = Mock {
            replies: replies.into(),
            sent: vec![],
        };
        assert_eq!(kx_read(&mut m).unwrap(), b);
        let mut m = Mock {
            replies: vec![kx_packet(0x4a, 1, 0, &[]).to_vec(); 40].into(),
            sent: vec![],
        };
        assert_eq!(kx_receive(&mut m, 0x44, 0).unwrap_err().code, "timeout");
        assert_eq!(m.replies.len(), 8);
    }
    #[test]
    fn kx_layer_deadline_is_passed_to_transport_without_ack_expansion() {
        struct DeadlineTransport {requested:Vec<Duration>}
        impl Transport for DeadlineTransport {
            fn send(&mut self,_:&[u8])->Result<()> {Ok(())}
            fn receive(&mut self)->Result<Vec<u8>> {unreachable!()}
            fn receive_for(&mut self,timeout:Duration)->Result<Vec<u8>> {
                self.requested.push(timeout);Ok(kx_packet(0x41,28,0,&[0;14]).to_vec())
            }
        }
        let mut t=DeadlineTransport{requested:vec![]};
        kx_receive_until(&mut t,0x41,0,Instant::now()+Duration::from_secs(10)).unwrap();
        assert!(t.requested[0]>Duration::from_secs(9));assert!(t.requested[0]<=Duration::from_secs(10));
        kx_receive(&mut t,0x41,0).unwrap();assert!(t.requested[1]<=Duration::from_millis(500));
    }
    #[test]
    fn kx_stale_and_partial_timeout() {
        let before = [0u8; 128];
        let mut m = Mock {
            replies: kx_replies(before).into(),
            sent: vec![],
        };
        let mut c = kx_change(before);
        c.expected_state = None;
        assert_eq!(kx_apply(&mut m, c).unwrap_err().code, "stale_state");
        assert_eq!(m.sent.len(), 1);
        let mut replies = kx_replies(before);
        replies.push(kx_packet(4, 10, 0, &[]).to_vec());
        let mut m = Mock {
            replies: replies.into(),
            sent: vec![],
        };
        assert_eq!(
            kx_apply(&mut m, kx_change(before)).unwrap_err().code,
            "timeout"
        );
        assert_eq!(m.sent.len(), 3);
    }
    #[test]
    fn kx_errors_distinguish_pre_read_ack_and_readback() {
        let before=[0u8;128];
        let mut m=Mock{replies:VecDeque::new(),sent:vec![]};
        let e=kx_apply(&mut m,kx_change(before)).unwrap_err();
        assert!(e.message.contains("尚未发送SET"));assert!(!m.sent.iter().any(|p|p[1]==4));
        let mut m=Mock{replies:kx_replies(before).into(),sent:vec![]};
        let e=kx_apply(&mut m,kx_change(before)).unwrap_err();
        assert!(e.message.contains("SET已发送，等待ACK失败"));
        let mut r=kx_replies(before);r.extend((0..10).map(|i|kx_packet(4,10,i,&[]).to_vec()));
        let mut m=Mock{replies:r.into(),sent:vec![]};
        let e=kx_apply(&mut m,kx_change(before)).unwrap_err();
        assert!(e.message.contains("SET已发送，完整读回失败"));
    }
    #[test]
    fn kx_performance_preserves_unknown_bytes_and_mac_clears_win_lock() {
        let mut before = [0xa7;128];
        before[1]=3; before[3]=2; before[15]=1; before[24]=4; before[27]=0;
        let mut after=before;
        after[1]=1; after[3]=0; after[15]=0; after[27]=2;
        let c=SettingsChange {report_rate_hz:Some(250),low_latency:Some(true),mac_mode:Some(true),expected_state:Some(hex::encode(before)),..Default::default()};
        let mut replies=kx_replies(before);
        replies.extend((0..10).map(|i|kx_packet(4,10,i,&[]).to_vec()));
        replies.extend(kx_replies(after));
        let mut m=Mock{replies:replies.into(),sent:vec![]};
        assert_eq!(kx_apply(&mut m,c).unwrap(),after);
    }
    #[test]
    fn kx_performance_rejects_mac_win_conflict_and_unknown_target_before_set() {
        let before=[0;128];
        let c=SettingsChange {mac_mode:Some(true),win_lock:Some(true),expected_state:Some(hex::encode(before)),..Default::default()};
        let mut m=Mock{replies:kx_replies(before).into(),sent:vec![]};
        assert_eq!(kx_apply(&mut m,c).unwrap_err().code,"invalid_parameter");
        assert_eq!(m.sent.len(),1);
        let c=SettingsChange {report_rate_hz:Some(1000),expected_state:Some(hex::encode(before)),..Default::default()};
        let mut m=Mock{replies:kx_replies(before).into(),sent:vec![]};
        assert_eq!(kx_apply(&mut m,c).unwrap_err().code,"unsupported");
        assert_eq!(m.sent.len(),1);
    }
    #[test]
    fn atk_rejects_keyboard_performance_options_without_io() {
        let mut m=Mock{replies:VecDeque::new(),sent:vec![]};
        let c=SettingsChange {low_latency:Some(true),..Default::default()};
        assert_eq!(apply(&mut m,c).unwrap_err().code,"unsupported");
        assert!(m.sent.is_empty());
    }
}
