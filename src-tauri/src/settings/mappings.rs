//! ATK single-button mappings, derived from official zt/U classes; no default reconstruction.
use super::{atk, err, HidTransport, Result, Transport};
use serde::{Deserialize, Serialize};
mod shortcuts;
const IDS: [&str; 6] = ["left", "right", "center", "side1", "side2", "bottom"];
const LABELS: [&str; 6] = ["左键", "右键", "中键", "侧键1", "侧键2", "底部键"];
const ACTIONS: [(&str, &str, [u8; 3]); 19] = [
    ("disabled", "禁用", [1, 0, 0]),
    ("leftClick", "左键", [1, 1, 0]),
    ("rightClick", "右键", [1, 2, 0]),
    ("middleClick", "中键", [1, 4, 0]),
    ("back", "后退", [1, 8, 0]),
    ("forward", "前进", [1, 16, 0]),
    ("dpiCycle", "DPI循环", [2, 1, 0]),
    ("dpiUp", "DPI增加", [2, 2, 0]),
    ("dpiDown", "DPI减少", [2, 3, 0]),
    ("wheelUp", "滚轮向上", [11, 1, 0]),
    ("wheelDown", "滚轮向下", [11, 2, 0]),
    ("reportRateCycle", "回报率切换", [7, 0, 0]),
    ("wheelLeft", "向左横滚", [3, 1, 0]),
    ("wheelRight", "向右横滚", [3, 2, 0]),
    ("profile1", "板载配置1", [9, 161, 0]),
    ("profile2", "板载配置2", [9, 162, 0]),
    ("profile3", "板载配置3", [9, 163, 0]),
    ("profile4", "板载配置4", [9, 164, 0]),
    ("profileCycle", "板载配置循环", [9, 255, 0]),
];
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MappingChange {
    button_id: String,
    action: String,
    expected_state: String,
    shortcut: Option<shortcuts::Shortcut>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MappingSnapshot {
    device_id: String,
    model: &'static str,
    sampled_at: u64,
    state_token: String,
    buttons: Vec<Button>,
    actions: Vec<Action>,
    unsupported: Vec<Unsupported>,
    shortcut_support: Option<shortcuts::Support>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Button {
    id: &'static str,
    label: &'static str,
    current_action: Option<&'static str>,
    current_label: &'static str,
    editable: bool,
    raw_hex: String,
    shortcut: Option<shortcuts::Shortcut>,
}
#[derive(Serialize)]
struct Action {
    id: &'static str,
    label: &'static str,
    category: &'static str,
}
fn category(id: &str) -> &'static str {
    if id.starts_with("dpi") {
        "dpi"
    } else if id.starts_with("wheel") {
        "wheel"
    } else if id.starts_with("profile") {
        "profile"
    } else if id == "reportRateCycle" {
        "special"
    } else {
        "mouse"
    }
}
#[derive(Serialize)]
struct Unsupported {
    id: &'static str,
    reason: &'static str,
}
fn packet(cmd: u8, address: u16, data: &[u8], len: u8) -> [u8; 17] {
    let mut p = [0u8; 17];
    p[0] = 8;
    p[1] = cmd;
    p[3..5].copy_from_slice(&address.to_be_bytes());
    p[5] = len;
    p[6..6 + data.len()].copy_from_slice(data);
    p[16] = 0x4du8.wrapping_sub(p[1..16].iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    p
}
fn read_pair(t: &mut impl Transport, address: u16) -> Result<[u8; 8]> {
    t.send(&packet(8, address, &[], 8))?;
    for _ in 0..4 {
        let p = t.receive()?;
        if p.len() < 2 || p[0] != 8 || p[1] != 8 {
            continue;
        }
        if p.len() != 17 || p[3..5] != address.to_be_bytes() || p[5] != 8 {
            return Err(err(
                "invalid_response",
                "按键映射响应长度、地址或数据长度不符",
            ));
        }
        if p[2] != 0 {
            return Err(err("device_rejected", "按键映射读取返回非零状态"));
        }
        return Ok(p[6..14].try_into().unwrap());
    }
    Err(err("timeout", "未收到匹配的按键映射响应"))
}
fn read(t: &mut impl Transport) -> Result<[u8; 24]> {
    let mut b = [0; 24];
    for i in 0..3 {
        b[i * 8..i * 8 + 8].copy_from_slice(&read_pair(t, 96 + i as u16 * 8)?);
    }
    Ok(b)
}
fn decode(raw: &[u8]) -> Option<(&'static str, &'static str)> {
    // The current official RI menu uses Mouse/Disable; retain the historical
    // class-0 disabled encoding too, without silently normalizing either.
    if raw == [0, 0, 0, 85] {
        return Some(("disabled", "禁用"));
    }
    ACTIONS
        .iter()
        .find(|(_, _, v)| {
            raw[..3] == v[..]
                && raw[3] == 85u8.wrapping_sub(v.iter().fold(0u8, |a, b| a.wrapping_add(*b)))
        })
        .map(|(id, label, _)| (*id, *label))
}
fn snapshot(id: &str, b: [u8; 24]) -> MappingSnapshot {
    MappingSnapshot {
        device_id: id.into(),
        model: "ATK Mouse 1K Dongle",
        sampled_at: super::SystemTime::now()
            .duration_since(super::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        state_token: hex::encode(b),
        buttons: IDS
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let current = decode(&b[i * 4..i * 4 + 4]);
                Button {
                    id,
                    label: LABELS[i],
                    current_action: current.map(|v| v.0),
                    current_label: current.map(|v| v.1).unwrap_or("未知映射（保留原值）"),
                    editable: current.is_some(),
                    raw_hex: hex::encode(&b[i * 4..i * 4 + 4]),
                    shortcut: None,
                }
            })
            .collect(),
        actions: ACTIONS
            .iter()
            .map(|(id, label, _)| Action {
                id,
                label,
                category: category(id),
            })
            .collect(),
        unsupported: vec![
            Unsupported {
                id: "shortcut",
                reason: "组合快捷键动作序列仍在核验，暂不写入",
            },
            Unsupported {
                id: "macro",
                reason: "宏录制不在本轮范围",
            },
        ],
        shortcut_support: None,
    }
}
fn apply(t: &mut impl Transport, c: MappingChange) -> Result<[u8; 24]> {
    let index = IDS
        .iter()
        .position(|id| *id == c.button_id)
        .ok_or_else(|| err("invalid_parameter", "未知物理按钮"))?;
    let values = ACTIONS
        .iter()
        .find(|(id, _, _)| *id == c.action)
        .map(|v| v.2)
        .ok_or_else(|| err("unsupported", "未证实的按键功能，禁止写入"))?;
    let before = read(t)?;
    if c.expected_state != hex::encode(before) {
        return Err(err("stale_state", "按键映射已变化，请刷新后重新预览"));
    }
    if decode(&before[index * 4..index * 4 + 4]).is_none() {
        return Err(err("unsupported", "当前按钮为未知映射，只读保留，禁止覆盖"));
    }
    let mut target = before;
    target[index * 4..index * 4 + 3].copy_from_slice(&values);
    target[index * 4 + 3] = 85u8.wrapping_sub(values.iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    if before == target {
        return Ok(before);
    }
    // Preserve every other entry exactly, including unknown/macro/shortcut mappings.
    t.send(&packet(
        7,
        96 + index as u16 * 4,
        &target[index * 4..index * 4 + 4],
        4,
    ))?;
    for _ in 0..3 {
        if read(t)? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "按键映射整块读回不一致；可能已写入，请刷新核对",
    ))
}
fn open(id: &str) -> Result<HidTransport> {
    let api = hidapi::HidApi::new().map_err(|e| err("connection", e.to_string()))?;
    let d = api
        .device_list()
        .find(|d| hex::encode(d.path().to_bytes()) == id && atk(d))
        .ok_or_else(|| {
            err(
                "unsupported",
                "此设备接口尚未核实按键映射协议（当前仅支持准确ATK接口）",
            )
        })?;
    Ok(HidTransport(
        d.open_device(&api)
            .map_err(|e| err("connection", e.to_string()))?,
    ))
}
pub(crate) fn read_sync(id: &str) -> Result<MappingSnapshot> {
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    shortcuts::read_snapshot(id, &mut t)
}
pub(crate) fn apply_sync(id: &str, change: MappingChange) -> Result<MappingSnapshot> {
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    shortcuts::apply_snapshot(id, &mut t, change)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
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
                .ok_or_else(|| err("timeout", "fixture timeout"))
        }
    }
    fn replies(b: [u8; 24]) -> Vec<Vec<u8>> {
        (0..3)
            .map(|i| packet(8, 96 + i * 8, &b[i as usize * 8..i as usize * 8 + 8], 8).to_vec())
            .collect()
    }
    fn change(b: [u8; 24]) -> MappingChange {
        MappingChange {
            button_id: "side1".into(),
            action: "back".into(),
            expected_state: hex::encode(b),
            shortcut: None,
        }
    }
    #[test]
    fn precise_get_and_set() {
        assert_eq!(
            packet(8, 96, &[], 8),
            [8, 8, 0, 0, 96, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 221]
        );
        let p = packet(7, 108, &[1, 8, 0, 76], 4);
        assert_eq!(&p[..10], &[8, 7, 0, 0, 108, 4, 1, 8, 0, 76]);
        assert_eq!(p[1..].iter().fold(0u8, |a, b| a.wrapping_add(*b)), 77);
    }
    #[test]
    fn unknown_entries_are_preserved() {
        let mut before = [0xee; 24];
        before[12..16].copy_from_slice(&[1, 16, 0, 68]);
        let mut after = before;
        after[12..16].copy_from_slice(&[1, 8, 0, 76]);
        let mut r = replies(before);
        r.push(vec![8, 7]);
        r.extend(replies(after));
        let mut m = Mock {
            replies: r.into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, change(before)).unwrap(), after);
        assert_eq!(m.sent.len(), 7);
        assert_eq!(m.sent[3][5], 4);
        assert_eq!(&m.sent[3][6..10], &[1, 8, 0, 76]);
        assert_eq!(
            snapshot("fixture", before)
                .buttons
                .iter()
                .filter(|b| !b.editable)
                .count(),
            5
        );
    }
    #[test]
    fn stale_does_not_write() {
        let b = [0xee; 24];
        let mut c = change(b);
        c.expected_state = "stale".into();
        let mut m = Mock {
            replies: replies(b).into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, c).unwrap_err().code, "stale_state");
        assert_eq!(m.sent.len(), 3);
    }
    #[test]
    fn whole_map_must_match() {
        let mut b = [0xee; 24];
        b[12..16].copy_from_slice(&[1, 16, 0, 68]);
        let mut r = replies(b);
        for _ in 0..3 {
            r.extend(replies(b));
        }
        let mut m = Mock {
            replies: r.into(),
            sent: vec![],
        };
        assert_eq!(
            apply(&mut m, change(b)).unwrap_err().code,
            "readback_mismatch"
        );
    }
    #[test]
    fn bad_envelope_and_unsupported_action() {
        let mut r = replies([0; 24]);
        r[0][5] = 4;
        let mut m = Mock {
            replies: r.into(),
            sent: vec![],
        };
        assert_eq!(read(&mut m).unwrap_err().code, "invalid_response");
        let mut c = change([0; 24]);
        c.action = "macro".into();
        assert_eq!(apply(&mut m, c).unwrap_err().code, "unsupported");
        assert_eq!(m.sent.len(), 1);
    }
}
