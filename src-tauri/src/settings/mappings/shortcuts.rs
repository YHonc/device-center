//! Bounded ATK shortcuts: max six official 3-byte down/up actions.
use super::*;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Shortcut {
    key_code: u8,
    modifiers: Vec<u8>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Support {
    max_modifiers: u8,
    max_actions: u8,
    key_code_min: u8,
    key_code_max: u8,
    modifiers: [u8; 8],
    keyboard_options: Vec<KeyOption>,
}
#[derive(Serialize)]
struct KeyOption {
    label: String,
    value: u8,
    category: &'static str,
}
const MEDIA: [(&str, &str, u8, u8); 20] = [
    ("brightnessUp", "亮度增加", 111, 0),
    ("brightnessDown", "亮度减少", 112, 0),
    ("mediaPlayer", "媒体播放器", 131, 1),
    ("stopPlayback", "停止播放", 183, 0),
    ("playPause", "播放/暂停", 205, 0),
    ("previousTrack", "上一曲", 182, 0),
    ("nextTrack", "下一曲", 181, 0),
    ("mute", "静音", 226, 0),
    ("volumeUp", "音量增加", 233, 0),
    ("volumeDown", "音量减少", 234, 0),
    ("mail", "邮件", 138, 1),
    ("homepage", "浏览器主页", 35, 2),
    ("search", "搜索", 33, 2),
    ("refresh", "刷新", 39, 2),
    ("favorites", "收藏夹", 42, 2),
    ("stopLoading", "停止加载", 38, 2),
    ("webForward", "浏览器前进", 37, 2),
    ("webBackward", "浏览器后退", 36, 2),
    ("computer", "计算器", 146, 1),
    ("myComputer", "我的电脑", 148, 1),
];
fn media(id: &str) -> Option<(&'static str, &'static str, u8, u8)> {
    MEDIA.iter().find(|a| a.0 == id).copied()
}
fn encode_sequence(actions: &[u8], original: [u8; 20]) -> [u8; 20] {
    let mut b = original;
    b[0] = (actions.len() / 3) as u8;
    b[1..1 + actions.len()].copy_from_slice(actions);
    b[actions.len() + 1] = 85u8.wrapping_sub(
        b[..actions.len() + 1]
            .iter()
            .fold(0u8, |a, b| a.wrapping_add(*b)),
    );
    b
}
fn encode_media(action: (&str, &str, u8, u8), original: [u8; 20]) -> [u8; 20] {
    encode_sequence(
        &[0x82, action.2, action.3, 0x42, action.2, action.3],
        original,
    )
}
fn decode_media(b: [u8; 20]) -> Option<(&'static str, &'static str, u8, u8)> {
    MEDIA
        .iter()
        .find(|a| encode_media(**a, b)[..8] == b[..8])
        .copied()
}
fn keyboard_options() -> Vec<KeyOption> {
    (4..=99)
        .filter(|v| *v != 50)
        .chain(std::iter::once(101))
        .map(|v| KeyOption {
            label: key_label(v),
            value: v,
            category: if (58..=69).contains(&v) || (104..=115).contains(&v) {
                "function"
            } else if (83..=99).contains(&v) {
                "keypad"
            } else {
                "keyboard"
            },
        })
        .chain((224..=231).map(|v| KeyOption {
            label: key_label(v),
            value: v,
            category: "modifier",
        }))
        .collect()
}
fn key_label(code: u8) -> String {
    if (4..=29).contains(&code) {
        return ((b'A' + code - 4) as char).to_string();
    }
    if (30..=38).contains(&code) {
        return (code - 29).to_string();
    }
    if (58..=69).contains(&code) {
        return format!("F{}", code - 57);
    }
    if (104..=115).contains(&code) {
        return format!("F{}", code - 91);
    }
    let label = match code {
        39 => "0",
        40 => "Enter",
        41 => "Escape",
        42 => "Backspace",
        43 => "Tab",
        44 => "Space",
        45 => "-",
        46 => "=",
        47 => "[",
        48 => "]",
        49 => "\\",
        50 => "Non-US #",
        51 => ";",
        52 => "'",
        53 => "`",
        54 => ",",
        55 => ".",
        56 => "/",
        57 => "Caps Lock",
        70 => "Print Screen",
        71 => "Scroll Lock",
        72 => "Pause",
        73 => "Insert",
        74 => "Home",
        75 => "Page Up",
        76 => "Delete",
        77 => "End",
        78 => "Page Down",
        79 => "→",
        80 => "←",
        81 => "↓",
        82 => "↑",
        83 => "Num Lock",
        84 => "小键盘 /",
        85 => "小键盘 *",
        86 => "小键盘 -",
        87 => "小键盘 +",
        88 => "小键盘 Enter",
        89 => "小键盘 1",
        90 => "小键盘 2",
        91 => "小键盘 3",
        92 => "小键盘 4",
        93 => "小键盘 5",
        94 => "小键盘 6",
        95 => "小键盘 7",
        96 => "小键盘 8",
        97 => "小键盘 9",
        98 => "小键盘 0",
        99 => "小键盘 .",
        101 => "菜单",
        224 => "左 Ctrl",
        225 => "左 Shift",
        226 => "左 Alt",
        227 => "左 Win",
        228 => "右 Ctrl",
        229 => "右 Shift",
        230 => "右 Alt",
        231 => "右 Win",
        _ => "未知键",
    };
    label.into()
}
#[derive(Clone, PartialEq, Debug)]
struct Raw {
    keys: [u8; 24],
    shortcuts: [[u8; 20]; 6],
}
impl Raw {
    fn token(&self) -> String {
        let mut b = self.keys.to_vec();
        for s in self.shortcuts {
            b.extend_from_slice(&s);
        }
        hex::encode(b)
    }
}
fn block(t: &mut impl Transport, address: u16) -> Result<[u8; 10]> {
    t.send(&packet(8, address, &[], 10))?;
    for _ in 0..4 {
        let p = t.receive()?;
        if p.len() < 2 || p[0] != 8 || p[1] != 8 {
            continue;
        }
        if p.len() != 17 || p[2] != 0 || p[3..5] != address.to_be_bytes() || p[5] != 10 {
            return Err(err("invalid_response", "快捷键区回复状态、长度或地址无效"));
        }
        return Ok(p[6..16].try_into().unwrap());
    }
    Err(err("timeout", "快捷键区读取超时"))
}
fn read_raw(t: &mut impl Transport) -> Result<Raw> {
    let mut r = Raw {
        keys: read(t)?,
        shortcuts: [[0; 20]; 6],
    };
    for i in 0..6 {
        for part in 0..2 {
            r.shortcuts[i][part * 10..part * 10 + 10]
                .copy_from_slice(&block(t, 256 + i as u16 * 32 + part as u16 * 10)?);
        }
    }
    Ok(r)
}
fn encode(s: &Shortcut, original: [u8; 20]) -> Result<[u8; 20]> {
    if (!((4..=99).contains(&s.key_code) && s.key_code != 50)
        && s.key_code != 101
        && !(224..=231).contains(&s.key_code))
        || ((224..=231).contains(&s.key_code) && !s.modifiers.is_empty())
        || s.modifiers.len() > 2
        || s.modifiers
            .iter()
            .any(|m| ![1, 2, 4, 8, 16, 32, 64, 128].contains(m))
        || s.modifiers
            .iter()
            .enumerate()
            .any(|(i, m)| s.modifiers[..i].contains(m))
    {
        return Err(err(
            "invalid_parameter",
            "请选择支持的键或单独修饰键，组合最多两个不同修饰键（最多6动作）",
        ));
    }
    let mut actions = Vec::new();
    for m in &s.modifiers {
        actions.extend_from_slice(&[0x80, *m, 0]);
    }
    let (kind, key) = if (224..=231).contains(&s.key_code) {
        (0, 1u8 << (s.key_code - 224))
    } else {
        (1, s.key_code)
    };
    actions.extend_from_slice(&[0x80 | kind, key, 0, 0x40 | kind, key, 0]);
    for m in s.modifiers.iter().rev() {
        actions.extend_from_slice(&[0x40, *m, 0]);
    }
    Ok(encode_sequence(&actions, original))
}
fn decode_shortcut(b: [u8; 20]) -> Option<Shortcut> {
    let n = b[0] as usize;
    if ![2, 4, 6].contains(&n) || b[..n * 3 + 2].iter().fold(0u8, |a, b| a.wrapping_add(*b)) != 85 {
        return None;
    }
    let mods = n / 2 - 1;
    let code = b[mods * 3 + 2];
    let s = Shortcut {
        key_code: if b[mods * 3 + 1] & 7 == 0 && code.count_ones() == 1 {
            224 + code.trailing_zeros() as u8
        } else {
            code
        },
        modifiers: (0..mods).map(|i| b[i * 3 + 2]).collect(),
    };
    let expected = encode(&s, b).ok()?;
    if expected[..n * 3 + 2] != b[..n * 3 + 2] {
        return None;
    }
    Some(s)
}
// U.setKey: class 6, zero-based macro slot, fixed repetition count.
// Special stop/loop modes 253..255 remain outside this editable subset.
fn known_macro(raw: &[u8]) -> bool {
    raw[0] == 6
        && raw[1] < 12
        && (1..=250).contains(&raw[2])
        && raw.iter().fold(0u8, |a, b| a.wrapping_add(*b)) == 85
}
fn view(id: &str, r: Raw) -> MappingSnapshot {
    let mut s = snapshot(id, r.keys);
    s.state_token = r.token();
    s.shortcut_support = Some(Support {
        max_modifiers: 2,
        max_actions: 6,
        key_code_min: 4,
        key_code_max: 101,
        modifiers: [1, 2, 4, 8, 16, 32, 64, 128],
        keyboard_options: keyboard_options(),
    });
    s.actions.push(Action {
        id: "shortcut",
        label: "组合快捷键",
        category: "shortcut",
    });
    s.actions
        .extend(MEDIA.iter().map(|(id, label, _, _)| Action {
            id,
            label,
            category: "media",
        }));
    s.unsupported.retain(|v| v.id != "shortcut");
    for i in 0..6 {
        if known_macro(&r.keys[i * 4..i * 4 + 4]) {
            s.buttons[i].editable = true;
            s.buttons[i].current_action = Some("macro");
            s.buttons[i].current_label = "宏（在宏编辑器中配置）";
        }
        if r.keys[i * 4..i * 4 + 4] == [5, 0, 0, 80] {
            if let Some(action) = decode_media(r.shortcuts[i]) {
                s.buttons[i].editable = true;
                s.buttons[i].current_action = Some(action.0);
                s.buttons[i].current_label = action.1;
            }
            if let Some(shortcut) = decode_shortcut(r.shortcuts[i]) {
                s.buttons[i].shortcut = Some(shortcut);
                s.buttons[i].editable = true;
                s.buttons[i].current_action = Some("shortcut");
                s.buttons[i].current_label = "组合快捷键";
            }
        }
    }
    s
}
pub(super) fn read_snapshot(id: &str, t: &mut impl Transport) -> Result<MappingSnapshot> {
    Ok(view(id, read_raw(t)?))
}
fn verify(t: &mut impl Transport, target: &Raw) -> Result<()> {
    for _ in 0..3 {
        if read_raw(t)? == *target {
            return Ok(());
        }
    }
    Err(err(
        "readback_mismatch",
        "完整按键及快捷键区读回不一致；可能已部分写入，请重新读取",
    ))
}
pub(super) fn apply_snapshot(
    id: &str,
    t: &mut impl Transport,
    c: MappingChange,
) -> Result<MappingSnapshot> {
    let i = IDS
        .iter()
        .position(|k| *k == c.button_id)
        .ok_or_else(|| err("invalid_parameter", "未知按钮"))?;
    // Validate the requested action before any transport query.
    let values = if media(&c.action).is_some() {
        if c.shortcut.is_some() {
            return Err(err("invalid_parameter", "媒体预设不可附快捷键参数"));
        }
        [5, 0, 0]
    } else if c.action == "shortcut" {
        encode(
            c.shortcut
                .as_ref()
                .ok_or_else(|| err("invalid_parameter", "缺少快捷键"))?,
            [0; 20],
        )?;
        [5, 0, 0]
    } else {
        if c.shortcut.is_some() {
            return Err(err("invalid_parameter", "普通功能不可附快捷键"));
        }
        ACTIONS
            .iter()
            .find(|a| a.0 == c.action)
            .ok_or_else(|| err("unsupported", "未知功能"))?
            .2
    };
    let before = read_raw(t)?;
    if c.expected_state != before.token() {
        return Err(err("stale_state", "按键或快捷键区已变化，请重新预览"));
    }
    if decode(&before.keys[i * 4..i * 4 + 4]).is_none()
        && !known_macro(&before.keys[i * 4..i * 4 + 4])
        && !(before.keys[i * 4..i * 4 + 4] == [5, 0, 0, 80]
            && (decode_shortcut(before.shortcuts[i]).is_some()
                || decode_media(before.shortcuts[i]).is_some()))
    {
        return Err(err("unsupported", "未知映射只读保留"));
    }
    let mut target = before.clone();
    if c.action == "shortcut" || media(&c.action).is_some() {
        target.shortcuts[i] = if let Some(a) = media(&c.action) {
            encode_media(a, before.shortcuts[i])
        } else {
            encode(c.shortcut.as_ref().unwrap(), before.shortcuts[i])?
        };
        if target.shortcuts != before.shortcuts {
            for part in 0..2 {
                t.send(&packet(
                    7,
                    256 + i as u16 * 32 + part as u16 * 10,
                    &target.shortcuts[i][part * 10..part * 10 + 10],
                    10,
                ))?;
            }
            verify(t, &target)?;
        }
    }
    target.keys[i * 4..i * 4 + 3].copy_from_slice(&values);
    target.keys[i * 4 + 3] = 85u8.wrapping_sub(values.iter().fold(0u8, |a, b| a.wrapping_add(*b)));
    if target.keys != before.keys {
        t.send(&packet(
            7,
            96 + i as u16 * 4,
            &target.keys[i * 4..i * 4 + 4],
            4,
        ))?;
        verify(t, &target)?;
    }
    Ok(view(id, target))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_c_exact_and_tail_preserved() {
        let s = Shortcut {
            key_code: 6,
            modifiers: vec![1],
        };
        let b = encode(&s, [0xee; 20]).unwrap();
        assert_eq!(&b[..13], &[4, 128, 1, 0, 129, 6, 0, 65, 6, 0, 64, 1, 0]);
        assert_eq!(b[14..], [0xee; 6]);
        assert_eq!(decode_shortcut(b), Some(s));
    }
    #[test]
    fn limits_and_unknown_sequences() {
        assert!(encode(
            &Shortcut {
                key_code: 6,
                modifiers: vec![1, 2, 4]
            },
            [0; 20]
        )
        .is_err());
        assert!(encode(
            &Shortcut {
                key_code: 6,
                modifiers: vec![1, 1]
            },
            [0; 20]
        )
        .is_err());
        assert!(decode_shortcut([0xee; 20]).is_none());
    }
    #[test]
    fn official_keyboard_catalog_round_trips_single_modifiers() {
        let options = keyboard_options();
        assert_eq!(options.len(), 104);
        for option in options {
            let shortcut = Shortcut {
                key_code: option.value,
                modifiers: vec![],
            };
            let bytes = encode(&shortcut, [0xee; 20]).unwrap();
            assert_eq!(decode_shortcut(bytes), Some(shortcut));
            assert_eq!(&bytes[8..], &[0xee; 12]);
        }
        let control = encode(
            &Shortcut {
                key_code: 228,
                modifiers: vec![],
            },
            [0; 20],
        )
        .unwrap();
        assert_eq!(&control[..7], &[2, 128, 16, 0, 64, 16, 0]);
        for code in [50, 104] {
            assert!(encode(
                &Shortcut {
                    key_code: code,
                    modifiers: vec![]
                },
                [0; 20]
            )
            .is_err());
        }
        assert!(encode(
            &Shortcut {
                key_code: 224,
                modifiers: vec![1]
            },
            [0; 20]
        )
        .is_err());
    }
    #[test]
    fn official_media_actions_preserve_high_usage_byte_and_tail() {
        for action in MEDIA {
            let bytes = encode_media(action, [0xee; 20]);
            assert_eq!(
                &bytes[..7],
                &[2, 130, action.2, action.3, 66, action.2, action.3]
            );
            assert_eq!(decode_media(bytes), Some(action));
            assert!(decode_shortcut(bytes).is_none());
            assert_eq!(&bytes[8..], &[0xee; 12]);
        }
    }
    #[test]
    fn known_macro_readable_and_special_modes_preserved() {
        let mut raw = Raw {
            keys: [0; 24],
            shortcuts: [[0; 20]; 6],
        };
        raw.keys[..4].copy_from_slice(&[6, 0, 1, 78]);
        let snapshot = view("fixture", raw);
        assert!(snapshot.buttons[0].editable);
        assert_eq!(snapshot.buttons[0].current_action, Some("macro"));
        assert!(!known_macro(&[6, 0, 253, 82]));
        assert!(!known_macro(&[6, 12, 1, 66]));
    }
}
#[cfg(test)]
mod transaction_tests {
    use super::*;
    use std::collections::VecDeque;
    struct Mock {
        r: VecDeque<Vec<u8>>,
        s: Vec<Vec<u8>>,
    }
    impl Transport for Mock {
        fn send(&mut self, p: &[u8]) -> Result<()> {
            self.s.push(p.to_vec());
            Ok(())
        }
        fn receive(&mut self) -> Result<Vec<u8>> {
            self.r
                .pop_front()
                .ok_or_else(|| err("timeout", "fixture timeout"))
        }
    }
    fn responses(r: &Raw) -> Vec<Vec<u8>> {
        let mut v = Vec::new();
        for i in 0..3 {
            v.push(packet(8, 96 + i as u16 * 8, &r.keys[i * 8..i * 8 + 8], 8).to_vec());
        }
        for i in 0..6 {
            for p in 0..2 {
                v.push(
                    packet(
                        8,
                        256 + i as u16 * 32 + p as u16 * 10,
                        &r.shortcuts[i][p * 10..p * 10 + 10],
                        10,
                    )
                    .to_vec(),
                );
            }
        }
        v
    }
    #[test]
    fn replace_known_macro_without_touching_shortcut_region() {
        let mut before = Raw {
            keys: [0xee; 24],
            shortcuts: [[0xee; 20]; 6],
        };
        before.keys[12..16].copy_from_slice(&[6, 0, 1, 78]);
        let mut after = before.clone();
        after.keys[12..16].copy_from_slice(&[1, 8, 0, 76]);
        let mut replies = responses(&before);
        replies.push(vec![8, 7]);
        replies.extend(responses(&after));
        let mut t = Mock {
            r: replies.into(),
            s: vec![],
        };
        let snapshot = apply_snapshot(
            "fixture",
            &mut t,
            MappingChange {
                button_id: "side1".into(),
                action: "back".into(),
                expected_state: before.token(),
                shortcut: None,
            },
        )
        .unwrap();
        assert_eq!(snapshot.state_token, after.token());
        assert_eq!(t.s.iter().filter(|p| p[1] == 7).count(), 1);
        assert_eq!(after.shortcuts, before.shortcuts);
    }
    #[test]
    fn verifies_shortcut_before_selector_preserving_unknown_bytes() {
        let mut before = Raw {
            keys: [0xee; 24],
            shortcuts: [[0xee; 20]; 6],
        };
        before.keys[12..16].copy_from_slice(&[1, 8, 0, 76]);
        let shortcut = Shortcut {
            key_code: 6,
            modifiers: vec![1],
        };
        let mut mid = before.clone();
        mid.shortcuts[3] = encode(&shortcut, before.shortcuts[3]).unwrap();
        let mut after = mid.clone();
        after.keys[12..16].copy_from_slice(&[5, 0, 0, 80]);
        let mut r = responses(&before);
        r.extend([vec![8, 7], vec![8, 7]]);
        r.extend(responses(&mid));
        r.push(vec![8, 7]);
        r.extend(responses(&after));
        let mut m = Mock {
            r: r.into(),
            s: vec![],
        };
        let c = MappingChange {
            button_id: "side1".into(),
            action: "shortcut".into(),
            expected_state: before.token(),
            shortcut: Some(shortcut),
        };
        let result = apply_snapshot("fixture", &mut m, c).unwrap();
        assert_eq!(result.state_token, after.token());
        let writes: Vec<_> = m.s.iter().filter(|p| p[1] == 7).collect();
        assert_eq!(writes.len(), 3);
        assert_eq!(&writes[0][3..5], &(352u16).to_be_bytes());
        assert_eq!(&writes[1][3..5], &(362u16).to_be_bytes());
        assert_eq!(&writes[2][3..5], &(108u16).to_be_bytes());
        assert_eq!(&after.shortcuts[0], &before.shortcuts[0]);
    }
    #[test]
    fn media_write_verifies_shortcut_before_mapping_and_can_be_replaced() {
        let mut before = Raw {
            keys: [0xee; 24],
            shortcuts: [[0xee; 20]; 6],
        };
        before.keys[12..16].copy_from_slice(&[1, 8, 0, 76]);
        let mut middle = before.clone();
        middle.shortcuts[3] = encode_media(media("homepage").unwrap(), before.shortcuts[3]);
        let mut after = middle.clone();
        after.keys[12..16].copy_from_slice(&[5, 0, 0, 80]);
        let mut replies = responses(&before);
        replies.extend(responses(&middle));
        replies.extend(responses(&after));
        let mut t = Mock {
            r: replies.into(),
            s: vec![],
        };
        let result = apply_snapshot(
            "fixture",
            &mut t,
            MappingChange {
                button_id: "side1".into(),
                action: "homepage".into(),
                expected_state: before.token(),
                shortcut: None,
            },
        )
        .unwrap();
        assert_eq!(result.buttons[3].current_action, Some("homepage"));
        assert!(result.buttons[3].editable);
        let writes: Vec<_> = t.s.iter().filter(|p| p[1] == 7).collect();
        assert_eq!(writes.len(), 3);
        assert_eq!(&writes[0][3..6], &[1, 96, 10]);
        assert_eq!(&writes[2][3..6], &[0, 108, 4]);
        let mut ordinary = after.clone();
        ordinary.keys[12..16].copy_from_slice(&[1, 8, 0, 76]);
        let mut replies = responses(&after);
        replies.extend(responses(&ordinary));
        t.r = replies.into();
        let replaced = apply_snapshot(
            "fixture",
            &mut t,
            MappingChange {
                button_id: "side1".into(),
                action: "back".into(),
                expected_state: after.token(),
                shortcut: None,
            },
        )
        .unwrap();
        assert_eq!(replaced.state_token, ordinary.token());
        assert_eq!(ordinary.shortcuts, after.shortcuts);
    }
}
