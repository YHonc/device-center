//! F1 DPI and macro EEPROM transactions derived from the retained official driver.
use super::{atk, err, HidTransport, Result, Transport};
use serde::{Deserialize, Serialize};

const BUTTONS: [&str; 6] = ["left", "right", "center", "side1", "side2", "bottom"];
const MACRO_SIZE: usize = 384;
const MAX_ACTIONS: usize = 70;

#[derive(Clone, Copy)]
struct Identity {
    cid: u8,
    mid: u8,
    model: &'static str,
    ultra: bool,
    min: u32,
    max: u32,
}
fn identity(cid: u8, mid: u8) -> Result<Identity> {
    let (model, ultra, min, max) = match (cid, mid) {
        (2, 36) => ("ATK F1 PRO", false, 50, 36000),
        (2, 37) => ("ATK F1 PRO MAX", false, 50, 36000),
        (2, 42) => ("ATK F1 Ultimate", true, 10, 42000),
        (2, 45) => ("ATK F1 Ultra", true, 10, 42000),
        (2, 50) => ("ATK F1 EXTREME", true, 10, 42000),
        (2, 56) => ("ATK F1 LEVIATAN", true, 10, 42000),
        (2, 1 | 2 | 104) => ("ATK Dragonfly F1 PRO", false, 100, 30000),
        (2, 3 | 4 | 105) => ("ATK Dragonfly F1 PRO MAX", false, 100, 30000),
        _ => {
            return Err(err(
                "unsupported",
                format!("尚未核实此鼠标CID/MID的扩展编码：{cid},{mid}"),
            ))
        }
    };
    Ok(Identity {
        cid,
        mid,
        model,
        ultra,
        min,
        max,
    })
}
fn packet(command: u8, address: u16, data: &[u8], len: u8) -> [u8; 17] {
    let mut p = [0; 17];
    p[0] = 8;
    p[1] = command;
    p[3..5].copy_from_slice(&address.to_be_bytes());
    p[5] = len;
    p[6..6 + data.len()].copy_from_slice(data);
    p[16] = 0x4du8.wrapping_sub(p[1..16].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    p
}
fn response(
    t: &mut impl Transport,
    command: u8,
    address: Option<u16>,
    len: usize,
) -> Result<Vec<u8>> {
    for _ in 0..4 {
        let p = t.receive()?;
        if p.len() < 2 || p[0] != 8 || p[1] != command {
            continue;
        }
        if p.len() != 17
            || address.is_some_and(|a| p[3..5] != a.to_be_bytes() || p[5] as usize != len)
        {
            return Err(err(
                "invalid_response",
                "ATK扩展回复长度、地址或数据长度不匹配",
            ));
        }
        if p[2] != 0 {
            return Err(err("device_rejected", "ATK扩展命令返回非零状态"));
        }
        return Ok(p[6..6 + len].to_vec());
    }
    Err(err("timeout", "未收到匹配的ATK扩展回复"))
}
fn identify(t: &mut impl Transport) -> Result<Identity> {
    // Dr.get() uses command 16 with a zero dataValidLen, not EEPROM GET.
    t.send(&packet(16, 0, &[], 0))?;
    let b = response(t, 16, None, 10)?;
    identity(b[0], b[1])
}
fn read(t: &mut impl Transport, address: u16, count: usize) -> Result<Vec<u8>> {
    let mut b = Vec::with_capacity(count);
    for offset in (0..count).step_by(10) {
        let len = (count - offset).min(10);
        t.send(&packet(8, address + offset as u16, &[], len as u8))?;
        b.extend(response(t, 8, Some(address + offset as u16), len)?);
    }
    Ok(b)
}
fn write_changes(t: &mut impl Transport, address: u16, before: &[u8], after: &[u8]) -> Result<()> {
    for offset in (0..after.len()).step_by(10) {
        let end = (offset + 10).min(after.len());
        if before[offset..end] != after[offset..end] {
            t.send(&packet(
                7,
                address + offset as u16,
                &after[offset..end],
                (end - offset) as u8,
            ))?;
            // One GET immediately follows each SET: at most its single ACK can
            // precede the GET response; never accumulate a whole slot of ACKs.
            if read(t, address + offset as u16, end - offset)? != after[offset..end] {
                return Err(err(
                    "readback_mismatch",
                    "宏分块读回不一致，可能已部分写入，请刷新",
                ));
            }
        }
    }
    Ok(())
}
fn open(id: &str) -> Result<HidTransport> {
    let api = hidapi::HidApi::new().map_err(|e| err("connection", e.to_string()))?;
    let d = api
        .device_list()
        .find(|d| hex::encode(d.path().to_bytes()) == id && atk(d))
        .ok_or_else(|| err("unsupported", "设备不在已核实的ATK配置接口白名单中"))?;
    Ok(HidTransport(
        d.open_device(&api)
            .map_err(|e| err("connection", e.to_string()))?,
    ))
}
fn token(i: Identity, b: &[u8]) -> String {
    format!("{:02x}{:02x}{}", i.cid, i.mid, hex::encode(b))
}
fn crc(b: &[u8]) -> u8 {
    85u8.wrapping_sub(b.iter().fold(0u8, |a, b| a.wrapping_add(*b)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DpiSnapshot {
    device_id: String,
    model: &'static str,
    sensor: &'static str,
    state_token: String,
    current_level: u8,
    level_count: u8,
    levels: Vec<DpiLevel>,
    min: u32,
    max: u32,
    step_segments: Vec<StepSegment>,
    verified_on_hardware: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DpiLevel {
    level: u8,
    x_dpi: u32,
    y_dpi: u32,
    editable: bool,
}
#[derive(Serialize)]
struct StepSegment {
    until: u32,
    step: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DpiChange {
    level: u8,
    x_dpi: u32,
    y_dpi: u32,
    expected_state: String,
}
fn decode_dpi(b: &[u8], ultra: bool) -> (u32, u32) {
    let decode = |lo: u8, flags: u8| {
        let base = lo as u32 + (((flags >> 2) & 3) as u32) * 256;
        let value = if ultra {
            if flags & 2 != 0 {
                base * 50 + 10050
            } else {
                (base + 1) * 10
            }
        } else {
            (base + 1) * 50
        };
        value * if flags & 1 != 0 { 2 } else { 1 }
    };
    (decode(b[0], b[2] & 15), decode(b[1], b[2] >> 4))
}
fn encode_dpi(i: Identity, x: u32, y: u32) -> Result<[u8; 4]> {
    let encode = |v: u32| -> Result<(u8, u8)> {
        let step = if i.ultra {
            if v <= 10000 {
                10
            } else if v <= 30000 {
                50
            } else {
                100
            }
        } else {
            if v <= 30000 {
                50
            } else {
                100
            }
        };
        if v < i.min || v > i.max || v % step != 0 {
            return Err(err(
                "invalid_parameter",
                format!("DPI需在{}至{}之间，当前区间步长{step}", i.min, i.max),
            ));
        }
        let (base, flag) = if i.ultra {
            if v <= 10000 {
                (v / 10 - 1, 0)
            } else if v <= 30000 {
                ((v - 10050) / 50, 2)
            } else {
                ((v / 2 - 10050) / 50, 3)
            }
        } else {
            if v <= 30000 {
                (v / 50 - 1, 0)
            } else {
                (v / 100 - 1, 1)
            }
        };
        Ok((base as u8, ((base >> 8) as u8) << 2 | flag))
    };
    let (a, af) = encode(x)?;
    let (b, bf) = encode(y)?;
    let mut out = [a, b, af | (bf << 4), 0];
    out[3] = crc(&out[..3]);
    Ok(out)
}
fn read_dpi(t: &mut impl Transport) -> Result<Vec<u8>> {
    let mut b = read(t, 0, 10)?;
    super::parse(&packet(8, 0, &b, 10))?;
    // Match Gt.get's two-level 8-byte blocks exactly.
    for address in [12, 20, 28, 36] {
        b.extend(read(t, address, 8)?);
    }
    Ok(b)
}
fn editable_dpi(i: Identity, raw: &[u8]) -> bool {
    let (x, y) = decode_dpi(raw, i.ultra);
    raw[3] == crc(&raw[..3]) && encode_dpi(i, x, y).is_ok_and(|encoded| encoded.as_slice() == raw)
}
fn dpi_snapshot(id: &str, i: Identity, b: &[u8]) -> DpiSnapshot {
    DpiSnapshot {
        device_id: id.into(),
        model: i.model,
        sensor: if i.ultra {
            "PAW3950Ultra"
        } else if i.max == 30000 {
            "PAW3395"
        } else {
            "PAW3950"
        },
        state_token: token(i, b),
        current_level: b[4],
        level_count: b[2],
        levels: (0..b[2])
            .map(|level| {
                let raw = &b[10 + level as usize * 4..14 + level as usize * 4];
                let (x, y) = decode_dpi(raw, i.ultra);
                DpiLevel {
                    level,
                    x_dpi: x,
                    y_dpi: y,
                    editable: editable_dpi(i, raw),
                }
            })
            .collect(),
        min: i.min,
        max: i.max,
        step_segments: if i.ultra {
            vec![
                StepSegment {
                    until: 10000,
                    step: 10,
                },
                StepSegment {
                    until: 30000,
                    step: 50,
                },
                StepSegment {
                    until: i.max,
                    step: 100,
                },
            ]
        } else {
            vec![
                StepSegment {
                    until: 30000,
                    step: 50,
                },
                StepSegment {
                    until: i.max,
                    step: 100,
                },
            ]
        },
        verified_on_hardware: false,
    }
}
fn apply_dpi(t: &mut impl Transport, i: Identity, c: DpiChange) -> Result<Vec<u8>> {
    let encoded = encode_dpi(i, c.x_dpi, c.y_dpi)?;
    let before = read_dpi(t)?;
    if c.expected_state != token(i, &before) {
        return Err(err("stale_state", "DPI原块已变化，请刷新并重新预览"));
    }
    if c.level >= before[2] {
        return Err(err("invalid_parameter", "DPI档位不在当前已有档位中"));
    }
    let offset = 10 + c.level as usize * 4;
    if !editable_dpi(i, &before[offset..offset + 4]) {
        return Err(err("unsupported", "当前DPI档位编码或校验未知，保留原值"));
    }
    let mut target = before.clone();
    target[offset..offset + 4].copy_from_slice(&encoded);
    if before == target {
        return Ok(before);
    }
    let pair = c.level as usize / 2;
    let start = 10 + pair * 8;
    t.send(&packet(
        7,
        12 + pair as u16 * 8,
        &target[start..start + 8],
        8,
    ))?;
    for _ in 0..3 {
        if read_dpi(t)? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "DPI完整原块读回不一致，设备可能已写入，请刷新",
    ))
}
pub(crate) fn read_dpi_sync(id: &str) -> Result<DpiSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let b = read_dpi(&mut t)?;
    Ok(dpi_snapshot(id, i, &b))
}
pub(crate) fn apply_dpi_sync(id: &str, c: DpiChange) -> Result<DpiSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let b = apply_dpi(&mut t, i, c)?;
    Ok(dpi_snapshot(id, i, &b))
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MacroAction {
    kind: String,
    state: String,
    code: u8,
    delay_ms: u16,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MacroChange {
    slot: u8,
    actions: Vec<MacroAction>,
    expected_state: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MacroBindingChange {
    slot: u8,
    button_id: String,
    repeat: u8,
    expected_state: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MacroSnapshot {
    device_id: String,
    model: &'static str,
    slot: u8,
    state_token: String,
    editable: bool,
    actions: Vec<MacroAction>,
    max_actions: usize,
    buttons: Vec<MacroButton>,
    verified_on_hardware: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MacroButton {
    id: &'static str,
    raw_hex: String,
}
fn macro_address(slot: u8) -> Result<u16> {
    if slot >= 12 {
        return Err(err("invalid_parameter", "宏槽位需为0至11"));
    }
    Ok(768 + slot as u16 * 384)
}
fn header() -> [u8; 31] {
    let mut b = [255; 31];
    b[0] = 8;
    b[1..9].fill(2);
    b
}
fn erased(b: &[u8]) -> bool {
    b.iter().all(|v| *v == 0) || b.iter().all(|v| *v == 255)
}
fn encode_actions(actions: &[MacroAction]) -> Result<Vec<u8>> {
    if actions.len() > MAX_ACTIONS {
        return Err(err("invalid_parameter", "一个宏最多70个动作"));
    }
    let mut bytes = Vec::new();
    let mut held = std::collections::HashSet::new();
    for (index, a) in actions.iter().enumerate() {
        let (kind, code) = match a.kind.as_str() {
            "key" if (4..=115).contains(&a.code) => (1, a.code),
            "modifier" if (224..=231).contains(&a.code) => (0, 1u8 << (a.code - 224)),
            "mouse" if [1, 2, 4, 8, 16].contains(&a.code) => (4, a.code),
            _ => return Err(err("invalid_parameter", "宏动作类型或HID键值不受支持")),
        };
        let key = (kind, code);
        let state = match a.state.as_str() {
            "down" if held.insert(key) => 128,
            "up" if held.remove(&key) => 64,
            _ => {
                return Err(err(
                    "invalid_parameter",
                    "宏按下/抬起必须匹配，不能重复按下或孤立抬起",
                ))
            }
        };
        if a.delay_ms == 0 && index + 1 != actions.len() {
            return Err(err("invalid_parameter", "非末动作延迟至少1毫秒"));
        }
        bytes.extend([state | kind, code, 0]);
        bytes.extend(a.delay_ms.to_be_bytes());
    }
    if !held.is_empty() {
        return Err(err("invalid_parameter", "宏结束前必须释放全部按键"));
    }
    Ok(bytes)
}
fn decode_actions(b: &[u8]) -> Option<Vec<MacroAction>> {
    if erased(&b[..MACRO_SIZE]) {
        return Some(vec![]);
    }
    if b[..31] != header() || b[31] as usize > MAX_ACTIONS {
        return None;
    }
    let end = 32 + b[31] as usize * 5;
    if b[end] != crc(&b[31..end]) {
        return None;
    }
    let mut actions = Vec::new();
    for r in b[32..end].chunks_exact(5) {
        if r[2] != 0 || r[0] & 0x38 != 0 {
            return None;
        }
        let (kind, code) = match r[0] & 7 {
            1 => ("key", r[1]),
            4 => ("mouse", r[1]),
            0 if r[1].count_ones() == 1 => ("modifier", 224 + r[1].trailing_zeros() as u8),
            _ => return None,
        };
        let state = match r[0] & 0xc0 {
            128 => "down",
            64 => "up",
            _ => return None,
        };
        actions.push(MacroAction {
            kind: kind.into(),
            state: state.into(),
            code,
            delay_ms: u16::from_be_bytes([r[3], r[4]]),
        });
    }
    encode_actions(&actions).ok()?;
    Some(actions)
}
fn read_macro(t: &mut impl Transport, slot: u8) -> Result<Vec<u8>> {
    let mut b = read(t, macro_address(slot)?, MACRO_SIZE)?;
    for address in [96, 104, 112] {
        b.extend(read(t, address, 8)?);
    }
    Ok(b)
}
fn macro_snapshot(id: &str, i: Identity, slot: u8, b: &[u8]) -> MacroSnapshot {
    let decoded = decode_actions(b);
    MacroSnapshot {
        device_id: id.into(),
        model: i.model,
        slot,
        state_token: token(i, b),
        editable: decoded.is_some(),
        actions: decoded.unwrap_or_default(),
        max_actions: MAX_ACTIONS,
        buttons: BUTTONS
            .iter()
            .enumerate()
            .map(|(n, id)| MacroButton {
                id,
                raw_hex: hex::encode(&b[MACRO_SIZE + n * 4..MACRO_SIZE + n * 4 + 4]),
            })
            .collect(),
        verified_on_hardware: false,
    }
}
fn apply_macro(t: &mut impl Transport, i: Identity, c: MacroChange) -> Result<Vec<u8>> {
    let address = macro_address(c.slot)?;
    let actions = encode_actions(&c.actions)?;
    let before = read_macro(t, c.slot)?;
    if token(i, &before) != c.expected_state {
        return Err(err("stale_state", "宏或按键映射已变化，请刷新后重新预览"));
    }
    if decode_actions(&before).is_none() {
        return Err(err("unsupported", "当前宏含未知字段或动作，保留原值"));
    }
    let mut target = before.clone();
    if erased(&before[..MACRO_SIZE]) {
        target[..31].copy_from_slice(&header());
    }
    target[31] = c.actions.len() as u8;
    let end = 32 + actions.len();
    target[32..end].copy_from_slice(&actions);
    target[end] = crc(&target[31..end]);
    if before == target {
        return Ok(before);
    }
    write_changes(t, address, &before[..MACRO_SIZE], &target[..MACRO_SIZE])?;
    for _ in 0..3 {
        if read_macro(t, c.slot)? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "宏与映射完整读回不一致，设备可能已写入，请刷新",
    ))
}
fn bind_macro(t: &mut impl Transport, i: Identity, c: MacroBindingChange) -> Result<Vec<u8>> {
    macro_address(c.slot)?;
    let index = BUTTONS
        .iter()
        .position(|id| *id == c.button_id)
        .ok_or_else(|| err("invalid_parameter", "未知物理按钮"))?;
    if c.repeat == 0 || c.repeat > 250 {
        return Err(err("invalid_parameter", "宏重复次数需为1至250"));
    }
    let before = read_macro(t, c.slot)?;
    if token(i, &before) != c.expected_state {
        return Err(err("stale_state", "宏或按键映射已变化，请刷新后重新预览"));
    }
    if decode_actions(&before).is_none_or(|a| a.is_empty()) {
        return Err(err("unsupported", "先写入可识别的非空宏，再绑定按钮"));
    }
    let offset = MACRO_SIZE + index * 4;
    let raw = &before[offset..offset + 4];
    let known = match raw[..3] {
        [0, 0, 0] | [7, 0, 0] => true,
        [1, value, 0] => [0, 1, 2, 4, 8, 16].contains(&value),
        [2, value, 0] => (1..=3).contains(&value),
        [11, value, 0] => (1..=2).contains(&value),
        [3, value, 0] => (1..=2).contains(&value),
        [9, value, 0] => [161, 162, 163, 164, 255].contains(&value),
        [6, slot, repeat] => slot < 12 && (1..=250).contains(&repeat),
        _ => false,
    };
    if !known || raw[3] != crc(&raw[..3]) {
        return Err(err(
            "unsupported",
            "目标按钮映射未知，先在按键页确认普通功能后再绑定",
        ));
    }
    let mut target = before.clone();
    target[offset..offset + 3].copy_from_slice(&[6, c.slot, c.repeat]);
    target[offset + 3] = crc(&target[offset..offset + 3]);
    if before == target {
        return Ok(before);
    }
    t.send(&packet(
        7,
        96 + index as u16 * 4,
        &target[offset..offset + 4],
        4,
    ))?;
    for _ in 0..3 {
        if read_macro(t, c.slot)? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "宏绑定完整读回不一致，设备可能已写入，请刷新",
    ))
}
pub(crate) fn read_macro_sync(id: &str, slot: u8) -> Result<MacroSnapshot> {
    macro_address(slot)?;
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let b = read_macro(&mut t, slot)?;
    Ok(macro_snapshot(id, i, slot, &b))
}
pub(crate) fn apply_macro_sync(id: &str, c: MacroChange) -> Result<MacroSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let slot = c.slot;
    let b = apply_macro(&mut t, i, c)?;
    Ok(macro_snapshot(id, i, slot, &b))
}
pub(crate) fn bind_macro_sync(id: &str, c: MacroBindingChange) -> Result<MacroSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let slot = c.slot;
    let b = bind_macro(&mut t, i, c)?;
    Ok(macro_snapshot(id, i, slot, &b))
}

// Ht (169/10) and Wt (10/2), using the official UI's finite choices.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PerformanceChange {
    key: String,
    value: u32,
    expected_state: String,
}
const DEBOUNCE: &[u32] = &[0, 1, 2, 4, 8, 15, 20];
const SLEEP: &[u32] = &[30, 60, 120, 180, 300, 1200, 1500, 1800];
fn performance_specs(i: Identity) -> Vec<(&'static str, &'static str, usize, Vec<u32>)> {
    vec![
        ("keyDebounceTime", "按键去抖（毫秒）", 0, DEBOUNCE.to_vec()),
        ("moveSync", "Motion Sync", 2, vec![0, 1]),
        ("sleepSeconds", "休眠时间（秒）", 4, SLEEP.to_vec()),
        ("lineCorrection", "直线修正", 6, vec![0, 1]),
        ("rippleCorrection", "波纹修正", 8, vec![0, 1]),
        (
            "liftOffDistance",
            "抬起高度",
            10,
            if !i.ultra && i.max == 36000 {
                vec![1, 2, 3]
            } else {
                vec![1, 2]
            },
        ),
    ]
}
fn read_performance(t: &mut impl Transport) -> Result<Vec<u8>> {
    let mut b = read(t, 169, 10)?;
    b.extend(read(t, 10, 2)?);
    Ok(b)
}
fn performance_value(key: &str, b: &[u8], offset: usize) -> u32 {
    b[offset] as u32 * if key == "sleepSeconds" { 10 } else { 1 }
}
fn performance_snapshot(id: &str, i: Identity, b: &[u8]) -> super::SettingsSnapshot {
    let mut s = super::snapshot(id, i.model, None);
    s.state_token = Some(token(i, b));
    s.fields.clear();
    for (key, label, offset, values) in performance_specs(i) {
        let value = performance_value(key, b, offset);
        let valid = b[offset + 1] == crc(&b[offset..offset + 1]) && values.contains(&value);
        let choices = values
            .iter()
            .map(|value| super::Choice {
                value: *value,
                label: match key {
                    "moveSync" | "lineCorrection" | "rippleCorrection" => {
                        if *value == 1 {
                            "开启".into()
                        } else {
                            "关闭".into()
                        }
                    }
                    "liftOffDistance" => match value {
                        1 => "1 mm".into(),
                        2 => "2 mm".into(),
                        _ => "0.7 mm".into(),
                    },
                    "keyDebounceTime" => format!("{value} ms"),
                    _ => format!("{value} 秒"),
                },
            })
            .collect();
        s.fields.push(super::field(
            key,
            label,
            Some(value),
            Some(choices),
            if valid {
                None
            } else {
                Some("当前字段编码或校验未知，保留原值")
            },
        ));
    }
    s
}
fn apply_performance(t: &mut impl Transport, i: Identity, c: PerformanceChange) -> Result<Vec<u8>> {
    let (_, _, offset, values) = performance_specs(i)
        .into_iter()
        .find(|v| v.0 == c.key)
        .ok_or_else(|| err("unsupported", "此性能参数尚未核实"))?;
    if !values.contains(&c.value) {
        return Err(err("invalid_parameter", "参数值不在当前型号的官方选项中"));
    }
    let before = read_performance(t)?;
    if token(i, &before) != c.expected_state {
        return Err(err("stale_state", "性能参数原块已变化，请刷新后重新预览"));
    }
    if before[offset + 1] != crc(&before[offset..offset + 1])
        || !values.contains(&performance_value(&c.key, &before, offset))
    {
        return Err(err("unsupported", "当前字段编码或校验未知，保留原值"));
    }
    let mut target = before.clone();
    target[offset] = (c.value / if c.key == "sleepSeconds" { 10 } else { 1 }) as u8;
    target[offset + 1] = crc(&target[offset..offset + 1]);
    if target == before {
        return Ok(before);
    }
    let address = if offset == 10 {
        10
    } else {
        169 + offset as u16
    };
    t.send(&packet(7, address, &target[offset..offset + 2], 2))?;
    for _ in 0..3 {
        if read_performance(t)? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "性能参数完整原块读回不一致，设备可能已写入，请刷新",
    ))
}
pub(crate) fn read_performance_sync(id: &str) -> Result<super::SettingsSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let b = read_performance(&mut t)?;
    Ok(performance_snapshot(id, i, &b))
}
pub(crate) fn apply_performance_sync(
    id: &str,
    c: PerformanceChange,
) -> Result<super::SettingsSnapshot> {
    let q = crate::device_queue(id);
    let _g = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let i = identify(&mut t)?;
    let b = apply_performance(&mut t, i, c)?;
    Ok(performance_snapshot(id, i, &b))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Memory {
        b: Vec<u8>,
        sent: Vec<Vec<u8>>,
        pending: Vec<u8>,
        acks: std::collections::VecDeque<Vec<u8>>,
        corrupt: bool,
    }
    impl Memory {
        fn new() -> Self {
            let mut b = vec![0; 6000];
            b[..10].copy_from_slice(&[1, 84, 4, 81, 1, 84, 0, 85, 0, 85]);
            let i = identity(2, 42).unwrap();
            for n in 0..8 {
                b[12 + n * 4..16 + n * 4].copy_from_slice(&encode_dpi(i, 800, 800).unwrap());
            }
            for n in 0..6 {
                b[96 + n * 4..100 + n * 4].copy_from_slice(&[1, 1, 0, 83]);
            }
            Self {
                b,
                sent: vec![],
                pending: vec![],
                acks: std::collections::VecDeque::new(),
                corrupt: false,
            }
        }
    }
    impl Transport for Memory {
        fn send(&mut self, p: &[u8]) -> Result<()> {
            self.sent.push(p.to_vec());
            let a = u16::from_be_bytes([p[3], p[4]]) as usize;
            let n = p[5] as usize;
            if p[1] == 7 {
                self.b[a..a + n].copy_from_slice(&p[6..6 + n]);
                self.acks
                    .push_back(packet(7, a as u16, &[], n as u8).to_vec());
                if self.corrupt {
                    self.b[0] = 2;
                    self.b[1] = 83;
                }
            }
            if p[1] == 8 {
                self.pending = packet(8, a as u16, &self.b[a..a + n], n as u8).to_vec();
            }
            Ok(())
        }
        fn receive(&mut self) -> Result<Vec<u8>> {
            Ok(self
                .acks
                .pop_front()
                .unwrap_or_else(|| self.pending.clone()))
        }
    }
    fn actions() -> Vec<MacroAction> {
        vec![
            MacroAction {
                kind: "key".into(),
                state: "down".into(),
                code: 4,
                delay_ms: 25,
            },
            MacroAction {
                kind: "key".into(),
                state: "up".into(),
                code: 4,
                delay_ms: 0,
            },
        ]
    }
    fn performance_fixture() -> Memory {
        let mut t = Memory::new();
        t.b[169..179].copy_from_slice(&[1, 84, 1, 84, 6, 79, 0, 85, 0, 85]);
        t.b[10..12].copy_from_slice(&[1, 84]);
        t
    }
    #[test]
    fn captured_awake_f1_pro_identity_settings_and_dpi_decode() {
        struct Captured(std::collections::VecDeque<Vec<u8>>, Vec<Vec<u8>>);
        impl Transport for Captured {
            fn send(&mut self, p: &[u8]) -> Result<()> {
                self.1.push(p.to_vec());
                Ok(())
            }
            fn receive(&mut self) -> Result<Vec<u8>> {
                self.0
                    .pop_front()
                    .ok_or_else(|| err("timeout", "captured fixture exhausted"))
            }
        }
        // GET16's observed dataValidLen is 3; its EEPROM-style request is zero.
        // Unique hardware bytes are omitted from this sanitized identity frame.
        let mut frames = vec![vec![8, 16, 0, 0, 0, 3, 2, 36, 0, 0, 0, 0, 0, 0, 0, 0, 0]];
        for raw in [
            "08080000000a0154045101540055005592",
            "080800000c08070700470f0f0037000087",
            "0808000014081f1f00173f3f00d700007f",
            "080800001c083f3f00d73f3f00d7000077",
            "0808000024083f3f00d73f3f00d700006f",
        ] {
            frames.push(hex::decode(raw).unwrap());
        }
        let mut t = Captured(frames.into(), vec![]);
        let i = identify(&mut t).unwrap();
        let raw = read_dpi(&mut t).unwrap();
        let s = dpi_snapshot("fixture", i, &raw);
        assert_eq!(s.model, "ATK F1 PRO");
        assert_eq!(s.current_level, 1);
        assert_eq!(s.level_count, 4);
        assert_eq!(
            s.levels
                .iter()
                .map(|l| (l.x_dpi, l.y_dpi, l.editable))
                .collect::<Vec<_>>(),
            vec![
                (400, 400, true),
                (800, 800, true),
                (1600, 1600, true),
                (3200, 3200, true)
            ]
        );
        assert_eq!(t.1.len(), 6);
        assert!(t.1.iter().all(|p| [8, 16].contains(&p[1])));
    }
    #[test]
    fn performance_cas_minimal_pair_and_unknown_non_target_preserved() {
        let mut t = performance_fixture();
        let i = identity(2, 36).unwrap();
        t.b[177..179].copy_from_slice(&[0xee, 0xdd]);
        let before = read_performance(&mut t).unwrap();
        let original = t.b.clone();
        let after = apply_performance(
            &mut t,
            i,
            PerformanceChange {
                key: "sleepSeconds".into(),
                value: 1800,
                expected_state: token(i, &before),
            },
        )
        .unwrap();
        assert_eq!(&after[4..6], &[180, 161]);
        assert_eq!(&after[8..10], &[0xee, 0xdd]);
        assert_eq!(&t.b[..173], &original[..173]);
        assert_eq!(&t.b[175..], &original[175..]);
        let writes: Vec<_> = t.sent.iter().filter(|p| p[1] == 7).collect();
        assert_eq!(writes.len(), 1);
        assert_eq!(&writes[0][3..8], &[0, 173, 2, 180, 161]);
        assert_eq!(
            apply_performance(
                &mut t,
                i,
                PerformanceChange {
                    key: "moveSync".into(),
                    value: 0,
                    expected_state: token(i, &before)
                }
            )
            .unwrap_err()
            .code,
            "stale_state"
        );
    }
    #[test]
    fn performance_lod_model_gate_and_unknown_target_refuse_writes() {
        let mut t = performance_fixture();
        let i = identity(2, 42).unwrap();
        let before = read_performance(&mut t).unwrap();
        assert_eq!(
            apply_performance(
                &mut t,
                i,
                PerformanceChange {
                    key: "liftOffDistance".into(),
                    value: 3,
                    expected_state: token(i, &before)
                }
            )
            .unwrap_err()
            .code,
            "invalid_parameter"
        );
        t.b[169] = 3;
        t.b[170] = 82;
        let before = read_performance(&mut t).unwrap();
        assert_eq!(
            apply_performance(
                &mut t,
                i,
                PerformanceChange {
                    key: "keyDebounceTime".into(),
                    value: 8,
                    expected_state: token(i, &before)
                }
            )
            .unwrap_err()
            .code,
            "unsupported"
        );
        assert!(t.sent.iter().all(|p| p[1] != 7));
    }
    #[test]
    fn performance_non_target_change_during_readback_fails() {
        struct Damage(Memory);
        impl Transport for Damage {
            fn send(&mut self, p: &[u8]) -> Result<()> {
                self.0.send(p)?;
                if p[1] == 7 {
                    self.0.b[177] = 1;
                    self.0.b[178] = 84;
                }
                Ok(())
            }
            fn receive(&mut self) -> Result<Vec<u8>> {
                self.0.receive()
            }
        }
        let mut t = Damage(performance_fixture());
        let i = identity(2, 36).unwrap();
        let before = read_performance(&mut t).unwrap();
        assert_eq!(
            apply_performance(
                &mut t,
                i,
                PerformanceChange {
                    key: "moveSync".into(),
                    value: 0,
                    expected_state: token(i, &before)
                }
            )
            .unwrap_err()
            .code,
            "readback_mismatch"
        );
    }
    #[test]
    fn dpi_sensor_boundaries_round_trip_and_reject_rounding() {
        for id in [identity(2, 36).unwrap(), identity(2, 42).unwrap()] {
            for v in [id.min, 800, 10000, 10050, 30000, id.max] {
                let e = encode_dpi(id, v, v).unwrap();
                assert_eq!(decode_dpi(&e, id.ultra), (v, v));
            }
            assert!(encode_dpi(id, 801, 800).is_err());
        }
        assert!(identity(255, 255).is_err());
        assert!(identity(2, 94).is_err());
    }
    #[test]
    fn dpi_full_cas_and_preserve_adjacent_slot() {
        let mut t = Memory::new();
        let i = identity(2, 42).unwrap();
        let before = read_dpi(&mut t).unwrap();
        let raw = t.b.clone();
        let after = apply_dpi(
            &mut t,
            i,
            DpiChange {
                level: 1,
                x_dpi: 1550,
                y_dpi: 1600,
                expected_state: token(i, &before),
            },
        )
        .unwrap();
        assert_eq!(&t.b[..16], &raw[..16]);
        assert_eq!(&t.b[20..], &raw[20..]);
        assert_eq!(decode_dpi(&after[14..18], true), (1550, 1600));
        let sets: Vec<_> = t.sent.iter().filter(|p| p[1] == 7).collect();
        assert_eq!(sets.len(), 1);
        assert_eq!(&sets[0][3..6], &[0, 12, 8]);
        let count = t.sent.len();
        let e = apply_dpi(
            &mut t,
            i,
            DpiChange {
                level: 1,
                x_dpi: 1800,
                y_dpi: 1800,
                expected_state: token(i, &before),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, "stale_state");
        assert!(t.sent[count..].iter().all(|p| p[1] != 7));
    }
    #[test]
    fn dpi_unrelated_change_in_readback_fails() {
        let mut t = Memory::new();
        let i = identity(2, 42).unwrap();
        let b = read_dpi(&mut t).unwrap();
        t.corrupt = true;
        assert_eq!(
            apply_dpi(
                &mut t,
                i,
                DpiChange {
                    level: 0,
                    x_dpi: 1200,
                    y_dpi: 1200,
                    expected_state: token(i, &b)
                }
            )
            .unwrap_err()
            .code,
            "readback_mismatch"
        );
    }
    #[test]
    fn macro_bytes_and_tail_preservation_then_bind() {
        let mut t = Memory::new();
        let i = identity(2, 42).unwrap();
        let b = read_macro(&mut t, 0).unwrap();
        let after = apply_macro(
            &mut t,
            i,
            MacroChange {
                slot: 0,
                actions: actions(),
                expected_state: token(i, &b),
            },
        )
        .unwrap();
        assert_eq!(&after[32..42], &[129, 4, 0, 0, 25, 65, 4, 0, 0, 0]);
        assert_eq!(&after[43..], &b[43..]);
        assert_eq!(decode_actions(&after), Some(actions()));
        let bound = bind_macro(
            &mut t,
            i,
            MacroBindingChange {
                slot: 0,
                button_id: "side1".into(),
                repeat: 1,
                expected_state: token(i, &after),
            },
        )
        .unwrap();
        assert_eq!(&bound[396..400], &[6, 0, 1, 78]);
        assert_eq!(&bound[..396], &after[..396]);
        assert_eq!(&bound[400..], &after[400..]);
    }
    #[test]
    fn macro_stale_and_unknown_refuse_writes() {
        let mut t = Memory::new();
        let i = identity(2, 42).unwrap();
        let b = read_macro(&mut t, 0).unwrap();
        t.b[96] = 7;
        assert_eq!(
            apply_macro(
                &mut t,
                i,
                MacroChange {
                    slot: 0,
                    actions: actions(),
                    expected_state: token(i, &b)
                }
            )
            .unwrap_err()
            .code,
            "stale_state"
        );
        t.b[768] = 9;
        let b = read_macro(&mut t, 0).unwrap();
        assert_eq!(
            apply_macro(
                &mut t,
                i,
                MacroChange {
                    slot: 0,
                    actions: actions(),
                    expected_state: token(i, &b)
                }
            )
            .unwrap_err()
            .code,
            "unsupported"
        );
        assert!(t.sent.iter().all(|p| p[1] != 7));
    }
    #[test]
    fn macro_rejects_held_keys_and_out_of_bounds_slots() {
        assert!(encode_actions(&actions()[..1]).is_err());
        assert!(macro_address(12).is_err());
        let mut a = actions();
        a[0].delay_ms = 0;
        assert!(encode_actions(&a).is_err());
    }
    #[test]
    fn seventy_actions_drain_set_acks_and_preserve_next_slot() {
        let mut t = Memory::new();
        let i = identity(2, 42).unwrap();
        let b = read_macro(&mut t, 0).unwrap();
        let mut a: Vec<_> = (0..35).flat_map(|_| actions()).collect();
        for action in &mut a {
            action.delay_ms = 25;
        }
        let next = t.b[1152..1536].to_vec();
        let after = apply_macro(
            &mut t,
            i,
            MacroChange {
                slot: 0,
                actions: a.clone(),
                expected_state: token(i, &b),
            },
        )
        .unwrap();
        assert_eq!(decode_actions(&after), Some(a));
        assert!(t.acks.is_empty());
        assert_eq!(&t.b[1152..1536], next);
        for (index, p) in t.sent.iter().enumerate().filter(|(_, p)| p[1] == 7) {
            assert_eq!(t.sent[index + 1][1], 8);
            assert_eq!(&p[3..6], &t.sent[index + 1][3..6]);
        }
    }
}
