//! KX75 remapping: preserve all 504 readable bytes except one explicit four-byte key.
//! The vendor's fixed eight-byte write tail is a request contract, not readback data.
use super::*;
const KEYS: [&str; 96] = [
    "Esc",
    "backquote",
    "Tab",
    "Caps",
    "Left_Shift",
    "Left_Ctrl",
    "space1",
    "one",
    "Q",
    "A",
    "Z",
    "Left_Win",
    "F1",
    "two",
    "W",
    "S",
    "X",
    "Left_Alt",
    "F2",
    "three",
    "E",
    "D",
    "C",
    "space2",
    "F3",
    "four",
    "R",
    "F",
    "V",
    "space3",
    "F4",
    "five",
    "T",
    "G",
    "B",
    "Space",
    "F5",
    "six",
    "Y",
    "H",
    "N",
    "space4",
    "F6",
    "seven",
    "U",
    "J",
    "M",
    "space5",
    "F7",
    "eight",
    "I",
    "K",
    "comma",
    "Fn",
    "F8",
    "nine",
    "O",
    "L",
    "dot",
    "Right_Ctrl",
    "F9",
    "zero",
    "P",
    "semicolon",
    "slash",
    "space6",
    "F10",
    "transverse",
    "left_bracket",
    "singlequotes",
    "space7",
    "space8",
    "F11",
    "equal",
    "right_bracket",
    "noChange0",
    "space9",
    "space10",
    "F12",
    "Backspace",
    "backslash",
    "Enter",
    "Right_Shift",
    "left",
    "Insert",
    "space11",
    "space12",
    "space13",
    "up",
    "down",
    "Delete",
    "Home",
    "PgUp",
    "PgDn",
    "space14",
    "right",
];
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Change {
    layer: String,
    key_id: String,
    action: String,
    expected_state: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    device_id: String,
    model: &'static str,
    layer: String,
    sampled_at: u64,
    state_token: String,
    buttons: Vec<Button>,
    actions: Vec<Action>,
    unsupported: Vec<Unavailable>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Button {
    id: &'static str,
    label: &'static str,
    current_action: Option<String>,
    current_label: String,
    editable: bool,
    raw_hex: String,
}
#[derive(Serialize)]
struct Action {
    id: String,
    label: String,
    category: &'static str,
}
#[derive(Serialize)]
struct Unavailable {
    id: &'static str,
    reason: &'static str,
}
fn layer_code(layer: &str) -> Result<u8> {
    match layer {
        "Default" => Ok(0),
        "Fn" => Ok(16),
        "Fn2" => Ok(32),
        _ => Err(err("invalid_parameter", "未知键盘层")),
    }
}
// Official DF / t3 / n3 / i3: copied four-byte values, no synthesized shortcut encoding.
const CATALOG: &[(&str, &str, &str, [u8; 4])] = &[
    ("key:Esc","Esc","base", [0, 0, 0, 41]),
    ("key:F1","F1","base", [0, 0, 0, 58]),
    ("key:F2","F2","base", [0, 0, 0, 59]),
    ("key:F3","F3","base", [0, 0, 0, 60]),
    ("key:F4","F4","base", [0, 0, 0, 61]),
    ("key:F5","F5","base", [0, 0, 0, 62]),
    ("key:F6","F6","base", [0, 0, 0, 63]),
    ("key:F7","F7","base", [0, 0, 0, 64]),
    ("key:F8","F8","base", [0, 0, 0, 65]),
    ("key:F9","F9","base", [0, 0, 0, 66]),
    ("key:F10","F10","base", [0, 0, 0, 67]),
    ("key:F11","F11","base", [0, 0, 0, 68]),
    ("key:F12","F12","base", [0, 0, 0, 69]),
    ("key:Print","Print","base", [0, 0, 0, 70]),
    ("key:Scroll","Scroll","base", [0, 0, 0, 71]),
    ("key:Pause","Pause","base", [0, 0, 0, 72]),
    ("key:Backquote","Backquote","base", [0, 0, 0, 53]),
    ("key:1","1","base", [0, 0, 0, 30]),
    ("key:2","2","base", [0, 0, 0, 31]),
    ("key:3","3","base", [0, 0, 0, 32]),
    ("key:4","4","base", [0, 0, 0, 33]),
    ("key:5","5","base", [0, 0, 0, 34]),
    ("key:6","6","base", [0, 0, 0, 35]),
    ("key:7","7","base", [0, 0, 0, 36]),
    ("key:8","8","base", [0, 0, 0, 37]),
    ("key:9","9","base", [0, 0, 0, 38]),
    ("key:0","0","base", [0, 0, 0, 39]),
    ("key:Minus","Minus","base", [0, 0, 0, 45]),
    ("key:Equal","Equal","base", [0, 0, 0, 46]),
    ("key:Backspace","Backspace","base", [0, 0, 0, 42]),
    ("key:Insert","Insert","base", [0, 0, 0, 73]),
    ("key:Home","Home","base", [0, 0, 0, 74]),
    ("key:PgUp","PgUp","base", [0, 0, 0, 75]),
    ("key:Num Lock","Num Lock","base", [0, 0, 0, 83]),
    ("key:Num /","Num /","base", [0, 0, 0, 84]),
    ("key:Num *","Num *","base", [0, 0, 0, 85]),
    ("key:Num -","Num -","base", [0, 0, 0, 86]),
    ("key:Tab","Tab","base", [0, 0, 0, 43]),
    ("key:Q","Q","base", [0, 0, 0, 20]),
    ("key:W","W","base", [0, 0, 0, 26]),
    ("key:E","E","base", [0, 0, 0, 8]),
    ("key:R","R","base", [0, 0, 0, 21]),
    ("key:T","T","base", [0, 0, 0, 23]),
    ("key:Y","Y","base", [0, 0, 0, 28]),
    ("key:U","U","base", [0, 0, 0, 24]),
    ("key:I","I","base", [0, 0, 0, 12]),
    ("key:O","O","base", [0, 0, 0, 18]),
    ("key:P","P","base", [0, 0, 0, 19]),
    ("key:Left_Bracket","Left Bracket","base", [0, 0, 0, 47]),
    ("key:Right_Bracket","Right Bracket","base", [0, 0, 0, 48]),
    ("key:Backslash","Backslash","base", [0, 0, 0, 49]),
    ("key:Delete","Delete","base", [0, 0, 0, 76]),
    ("key:End","End","base", [0, 0, 0, 77]),
    ("key:PgDn","PgDn","base", [0, 0, 0, 78]),
    ("key:Num 7","Num 7","base", [0, 0, 0, 95]),
    ("key:Num 8","Num 8","base", [0, 0, 0, 96]),
    ("key:Num 9","Num 9","base", [0, 0, 0, 97]),
    ("key:Num +","Num +","base", [0, 0, 0, 87]),
    ("key:Caps","Caps","base", [0, 0, 0, 57]),
    ("key:A","A","base", [0, 0, 0, 4]),
    ("key:S","S","base", [0, 0, 0, 22]),
    ("key:D","D","base", [0, 0, 0, 7]),
    ("key:F","F","base", [0, 0, 0, 9]),
    ("key:G","G","base", [0, 0, 0, 10]),
    ("key:H","H","base", [0, 0, 0, 11]),
    ("key:J","J","base", [0, 0, 0, 13]),
    ("key:K","K","base", [0, 0, 0, 14]),
    ("key:L","L","base", [0, 0, 0, 15]),
    ("key:Semicolon","Semicolon","base", [0, 0, 0, 51]),
    ("key:Quote","Quote","base", [0, 0, 0, 52]),
    ("key:Enter","Enter","base", [0, 0, 0, 40]),
    ("key:Num 4","Num 4","base", [0, 0, 0, 92]),
    ("key:Num 5","Num 5","base", [0, 0, 0, 93]),
    ("key:Num 6","Num 6","base", [0, 0, 0, 94]),
    ("key:Left_Shift","Left Shift","base", [0, 2, 0, 0]),
    ("key:Z","Z","base", [0, 0, 0, 29]),
    ("key:X","X","base", [0, 0, 0, 27]),
    ("key:C","C","base", [0, 0, 0, 6]),
    ("key:V","V","base", [0, 0, 0, 25]),
    ("key:B","B","base", [0, 0, 0, 5]),
    ("key:N","N","base", [0, 0, 0, 17]),
    ("key:M","M","base", [0, 0, 0, 16]),
    ("key:Comma","Comma","base", [0, 0, 0, 54]),
    ("key:Dot","Dot","base", [0, 0, 0, 55]),
    ("key:Slash","Slash","base", [0, 0, 0, 56]),
    ("key:Right_Shift","Right Shift","base", [0, 32, 0, 0]),
    ("key:Up","Up","base", [0, 0, 0, 82]),
    ("key:Num 1","Num 1","base", [0, 0, 0, 89]),
    ("key:Num 2","Num 2","base", [0, 0, 0, 90]),
    ("key:Num 3","Num 3","base", [0, 0, 0, 91]),
    ("key:Num Enter","Num Enter","base", [0, 0, 0, 88]),
    ("key:Left_Ctrl","Left Ctrl","base", [0, 1, 0, 0]),
    ("key:Left_Win","Left Win","base", [0, 8, 0, 0]),
    ("key:Left_Alt","Left Alt","base", [0, 4, 0, 0]),
    ("key:Space","Space","base", [0, 0, 0, 44]),
    ("key:Right_Alt","Right Alt","base", [0, 64, 0, 0]),
    ("key:Right_Win","Right Win","base", [0, 128, 0, 0]),
    ("key:Menu","Menu","base", [0, 0, 0, 101]),
    ("key:Right_Ctrl","Right Ctrl","base", [0, 16, 0, 0]),
    ("key:Left","Left","base", [0, 0, 0, 80]),
    ("key:Down","Down","base", [0, 0, 0, 81]),
    ("key:Right","Right","base", [0, 0, 0, 79]),
    ("key:Num 0","Num 0","base", [0, 0, 0, 98]),
    ("key:Num Del","Num Del","base", [0, 0, 0, 99]),
    ("mouse:left","鼠标左键","mouse", [1, 1, 1, 0]),
    ("mouse:right","鼠标右键","mouse", [1, 2, 1, 0]),
    ("mouse:mid","中键","mouse", [1, 3, 1, 0]),
    ("mouse:mForward","鼠标前进","mouse", [1, 5, 1, 0]),
    ("mouse:mRetreat","鼠标后退","mouse", [1, 4, 1, 0]),
    ("mouse:scrollUp","滚轮上","mouse", [1, 8, 1, 0]),
    ("mouse:scrollDown","滚轮下","mouse", [1, 9, 1, 0]),
    ("media:playPause","播放/暂停","media", [2, 0, 0, 205]),
    ("media:silence","静音","media", [2, 0, 0, 226]),
    ("media:stopPlaying","停止播放","media", [2, 0, 0, 183]),
    ("media:previousSong","上一曲","media", [2, 0, 0, 182]),
    ("media:nextSong","下一曲","media", [2, 0, 0, 181]),
    ("media:volumeUp","音量+","media", [2, 0, 0, 233]),
    ("media:volumeDown","音量-","media", [2, 0, 0, 234]),
    ("extended:zoomOut","缩小","extended", [0, 1, 45, 0]),
    ("extended:zoomIn","放大","extended", [0, 1, 46, 0]),
    ("extended:resetting","复位","extended", [0, 1, 39, 0]),
    ("extended:recover","恢复","extended", [0, 1, 28, 0]),
    ("extended:repeal","撤销","extended", [0, 1, 29, 0]),
    ("extended:selectAll","全选","extended", [0, 1, 4, 0]),
    ("extended:new","新建","extended", [0, 1, 17, 0]),
    ("extended:copy","复制","extended", [0, 1, 6, 0]),
    ("extended:cut","剪切","extended", [0, 1, 27, 0]),
    ("extended:paste","粘贴","extended", [0, 1, 25, 0]),
    ("extended:save","保存","extended", [0, 1, 22, 0]),
    ("extended:open","打开","extended", [0, 1, 18, 0]),
    ("extended:newItem","新建项","extended", [0, 1, 23, 0]),
    ("extended:closeItem","关闭项","extended", [0, 1, 26, 0]),
    ("extended:retreat","后退","extended", [0, 4, 80, 0]),
    ("extended:forward","前进","extended", [0, 4, 79, 0]),
    ("extended:taskbar","任务栏","extended", [0, 8, 43, 0]),
    ("extended:openNarrative","打开叙述","extended", [0, 9, 88, 0]),
    ("extended:loopApplication","循环任务栏应用程序","extended", [0, 8, 23, 0]),
    ("extended:actionCenter","行动中心","extended", [0, 8, 4, 0]),
    ("extended:fileExplorer","文件资源管理器","extended", [0, 8, 8, 0]),
    ("extended:winSettingsCenter","Windows设置中心","extended", [0, 8, 12, 0]),
    ("extended:lockComputer","锁定计算机","extended", [0, 8, 15, 0]),
    ("extended:projectToOtherDevices","投屏到其他设备","extended", [0, 8, 14, 0]),
    ("extended:jumpToTray","跳转到托盘","extended", [0, 8, 5, 0]),
    ("extended:launchXboxGameBar","启动xbox游戏栏","extended", [0, 8, 10, 0]),
    ("extended:operation","运行","extended", [0, 8, 21, 0]),
    ("extended:search","搜索","extended", [0, 8, 22, 0]),
    ("extended:displaySetting","显示设置","extended", [0, 8, 24, 0]),
    ("extended:simpleMenu","简易菜单","extended", [0, 8, 27, 0]),
    ("extended:emojiBox","表情框","extended", [0, 8, 55, 0]),
    ("extended:startMenu","开始菜单","extended", [0, 1, 0, 41]),
    ("extended:conversionWindow","转换窗口","extended", [0, 4, 43, 0]),
    ("extended:closeWindow","关闭窗口","extended", [0, 4, 61, 0]),
    ("extended:endApplication","结束应用","extended", [0, 4, 61, 0]),
    ("extended:switchNextApplication","切换到下一个应用","extended", [0, 4, 41, 0]),
    ("extended:browserHome","浏览器主页","extended", [2, 0, 2, 35]),
    ("extended:calculator","计算器","extended", [2, 0, 1, 146]),
    ("extended:email","邮箱","extended", [2, 0, 1, 138]),
    ("extended:myComputer","我的电脑","extended", [2, 0, 1, 148]),
    ("extended:favorites","收藏","extended", [2, 0, 2, 42]),
    ("extended:showDesktop","显示桌面","extended", [0, 8, 7, 0]),
    ("security","Windows安全屏幕（Ctrl+Alt+Delete）","extended", [0, 5, 76, 0]),
    ("taskManager","任务管理器（Ctrl+Shift+Esc）","extended", [0, 3, 41, 0]),
    ("extended:screenBrightUp","屏幕亮度+","extended", [2, 0, 0, 111]),
    ("extended:screenBrightDown","屏幕亮度-","extended", [2, 0, 0, 112]),
    ("extended:fn","Fn","extended", [13, 0, 0, 0]),
    ("extended:fn2","Fn2","extended", [13, 1, 0, 0]),
    ("disabled","禁用","extended", [0, 0, 0, 0]),
];
fn action(id: &str) -> Result<[u8; 4]> {
    CATALOG.iter().find(|a| a.0 == id).map(|a| a.3)
        .ok_or_else(|| err("unsupported", "未知或未核实的键盘功能"))
}
fn actions() -> Vec<Action> {
    CATALOG.iter().map(|a| Action {id:a.0.into(),label:a.1.into(),category:a.2}).collect()
}
fn decode(raw: &[u8]) -> Option<Action> {
    if let Some(a)=CATALOG.iter().find(|a| a.3.as_slice()==raw) {
        return Some(Action{id:a.0.into(),label:a.1.into(),category:a.2});
    }
    // Exact additional Fn-layer values from KX75 _vt, not exposed as assignable actions.
    const NATIVE: &[[u8;4]]=&[
        [0,8,0,43],[0,8,0,8],
        [7,0,0,1],[7,0,0,4],[7,0,0,5],[7,0,0,6],[7,0,0,7],[7,0,0,8],
        [7,0,0,10],[7,0,0,17],[7,0,0,24],[7,0,0,26],
        [8,0,0,0],[8,2,0,0],[8,3,1,0],[8,3,2,0],[8,4,1,0],[8,4,2,0],
    ];
    NATIVE.iter().find(|v|v.as_slice()==raw).map(|_|Action {
        id:format!("native:{}",hex::encode(raw)),label:"原厂Fn层功能".into(),category:"native",
    })
}
fn packet(cmd: u8, total: u8, seq: u8, yy: u8, data: &[u8]) -> [u8; 20] {
    let mut p = kx_packet(cmd, total, seq, data);
    p[4] = yy;
    p[19] = p[..19].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    p
}
fn read(t: &mut impl Transport, layer: &str) -> Result<[u8; 504]> {
    let base = layer_code(layer)?;
    t.send(&packet(0x41, 1, 0, base, &[]))?;
    // Official getKeySetting uses one GET and a 10-second deadline for the complete layer.
    let deadline=Instant::now()+Duration::from_secs(10);
    let mut bytes = Vec::with_capacity(504);
    for seq in 0..36 {
        let p = kx_receive_until(t, 0x41, seq,deadline)?;
        if p[2] != 36 {
            return Err(err("invalid_response", format!("KX75键表分包数量不符合已捕获504字节契约：total={} seq={} YY={}",p[2],p[3],p[4])));
        }
        bytes.extend_from_slice(&p[5..19]);
    }
    Ok(bytes.try_into().unwrap())
}
fn write_payload(readable:[u8;504])->[u8;512] {
    let mut payload=[0u8;512];
    payload[..504].copy_from_slice(&readable);
    // A8.KX75.commonObj=$vt: space15=126 fixed zero bytes, fix=[90,165].
    // GET returns the first 120 space15 bytes; retain those and append only the unreturned tail.
    payload[504..].copy_from_slice(&[0,0,0,0,0,0,90,165]);
    payload
}
fn view(id: &str, layer: &str, b: [u8; 504]) -> Snapshot {
    Snapshot {
        device_id: id.into(),
        model: "MCHOSE KX75",
        layer: layer.into(),
        sampled_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        state_token: hex::encode(b),
        buttons: KEYS
            .into_iter()
            .enumerate()
            .filter(|(_, id)| !id.starts_with("space") && *id != "noChange0" && *id != "Fn")
            .map(|(i, id)| {
                let raw = &b[i * 4..i * 4 + 4];
                let a = decode(raw);
                Button {
                    id,
                    label: id,
                    current_action: a.as_ref().map(|a| a.id.clone()),
                    current_label: a
                        .as_ref()
                        .map(|a| a.label.clone())
                        .unwrap_or("未知或保留映射（只读）".into()),
                    editable: a.is_some(),
                    raw_hex: hex::encode(raw),
                }
            })
            .collect(),
        actions: actions(),
        unsupported: vec![
            Unavailable {
                id: "arbitraryShortcut",
                reason: "任意组合键4字节生成链尚未核实，仅开放官网确证预设",
            },
            Unavailable {
                id: "macro",
                reason: "完整宏区读取、索引和读回契约待核实",
            },
        ],
    }
}
fn apply(t: &mut impl Transport, c: Change) -> Result<[u8; 504]> {
    let base = layer_code(&c.layer)?;
    let new = action(&c.action)?;
    if c.layer != "Default" && new[0] == 13 {
        return Err(err("unsupported", "官网Fn/Fn2层不允许分配Fn或Fn2功能"));
    }
    let i = KEYS
        .iter()
        .position(|k| *k == c.key_id && !k.starts_with("space") && *k != "noChange0" && *k != "Fn")
        .ok_or_else(|| err("invalid_parameter", "未知或保留物理键"))?;
    let before = read(t, &c.layer).map_err(|e|kx_stage(e,"应用前读取键表失败（尚未发送SET）"))?;
    if c.expected_state != hex::encode(before) {
        return Err(err("stale_state", "该层完整键表已变化，请重新读取预览"));
    }
    if decode(&before[i * 4..i * 4 + 4]).is_none() {
        return Err(err("unsupported", "未知原始映射只读保留"));
    }
    let mut target = before;
    target[i * 4..i * 4 + 4].copy_from_slice(&new);
    if target == before {
        return Ok(before);
    }
    for (seq, chunk) in write_payload(target).chunks(14).enumerate() {
        // Official getWirelessYY reverses a COPY of the unpadded chunk. Tail A5 => trailingZeros=0.
        let yy = if seq == 36 {
            let zeros = chunk.iter().rev().take_while(|v| **v == 0).count();
            if zeros == chunk.len() {
                0
            } else {
                base + 14 - zeros as u8
            }
        } else {
            base + 14
        };
        t.send(&packet(1, 37, seq as u8, yy, chunk)).map_err(|e|kx_stage(e,"键表SET发送失败，可能部分写入，请刷新"))?;
        kx_receive(t, 1, seq as u8).map_err(|e|kx_stage(e,"键表SET已发送，等待ACK失败，请刷新"))?;
    }
    for _ in 0..2 {
        if read(t, &c.layer).map_err(|e|kx_stage(e,"键表SET已发送，完整读回失败，请刷新"))? == target {
            return Ok(target);
        }
    }
    Err(err(
        "readback_mismatch",
        "KX75该层完整键表读回不一致，可能已部分写入，请刷新",
    ))
}
pub(super) fn open(id: &str) -> Result<KxTransport> {
    let api = hidapi::HidApi::new().map_err(|e| err("connection", e.to_string()))?;
    let d = api
        .device_list()
        .find(|d| {
            hex::encode(d.path().to_bytes()) == id
                && d.vendor_id() == 0x3554
                && d.product_id() == 0xfa09
                && d.usage_page() == 0xff02
                && d.usage() == 2
                && d.interface_number() == 1
        })
        .ok_or_else(|| err("unsupported", "未匹配KX75精确接口"))?;
    let device = d
        .open_device(&api)
        .map_err(|e| err("connection", e.to_string()))?;
    crate::verify_kx75(&device).map_err(|e| err("unsupported", e))?;
    Ok(KxTransport { device })
}
pub(crate) fn read_sync(id: &str, layer: &str) -> Result<Snapshot> {
    layer_code(layer)?;
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    Ok(view(id, layer, read(&mut t, layer).map_err(|e|kx_stage(e,"读取键表失败（未发送设置写入）"))?))
}
pub(crate) fn apply_sync(id: &str, c: Change) -> Result<Snapshot> {
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = open(id)?;
    let layer = c.layer.clone();
    Ok(view(id, &layer, apply(&mut t, c)?))
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
    fn raw() -> [u8; 504] {
        let mut r = [0xee; 504];
        r[36..40].copy_from_slice(&[0, 0, 0, 4]);
        r
    }
    fn replies(raw: [u8; 504]) -> Vec<Vec<u8>> {
        raw.chunks(14)
            .enumerate()
            .map(|(s, c)| packet(0x41, 36, s as u8, 14, c).to_vec())
            .collect()
    }
    fn change(r: [u8; 504]) -> Change {
        Change {
            layer: "Default".into(),
            key_id: "A".into(),
            action: "taskManager".into(),
            expected_state: hex::encode(r),
        }
    }
    #[test]
    fn keyboard_codes_and_layout() {
        assert_eq!(KEYS[9], "A");
        assert_eq!(action("key:C").unwrap(), [0, 0, 0, 6]);
        assert_eq!(action("taskManager").unwrap(), [0, 3, 41, 0]);
        assert_eq!(packet(0x41, 1, 0, 16, &[])[19], 0x65);
    }
    #[test]
    fn official_catalog_decodes_non_letter_keys_and_native_layers() {
        assert_eq!(action("key:Esc").unwrap(),[0,0,0,41]);
        assert_eq!(action("key:Left_Shift").unwrap(),[0,2,0,0]);
        assert_eq!(action("mouse:left").unwrap(),[1,1,1,0]);
        assert_eq!(action("media:playPause").unwrap(),[2,0,0,205]);
        assert_eq!(action("extended:copy").unwrap(),[0,1,6,0]);
        assert_eq!(action("disabled").unwrap(),[0,0,0,0]);
        assert!(decode(&[0,0,0,58]).is_some());
        assert!(decode(&[7,0,0,4]).is_some());
        assert!(decode(&[0xfe,0,0,4]).is_none());
        assert!(action("native:07000004").is_err());
    }
    #[test]
    fn fn_assignment_is_blocked_on_fn_layers_without_io() {
        let mut c=change(raw());c.layer="Fn".into();c.action="extended:fn2".into();
        let mut m=Mock{sent:vec![],replies:VecDeque::new()};
        assert_eq!(apply(&mut m,c).unwrap_err().code,"unsupported");
        assert!(m.sent.is_empty());
    }
    #[test]
    fn whole_block_preserved_and_last_packet_exact() {
        let b = raw();
        let mut target = b;
        target[36..40].copy_from_slice(&[0, 3, 41, 0]);
        let mut r = replies(b);
        r.extend((0..37).map(|s| packet(1, 37, s, 14, &[]).to_vec()));
        r.extend(replies(target));
        let mut m = Mock {
            replies: r.into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, change(b)).unwrap(), target);
        let last = &m.sent[37];
        assert_eq!(&last[..5], &[19, 1, 37, 36, 14]);
        assert_eq!(&last[5..13], &[0,0,0,0,0,0,90,165]);
        assert_eq!(&last[13..19], &[0; 6]);
        let written:Vec<_>=m.sent.iter().filter(|p|p[1]==1).flat_map(|p|p[5..19].iter().copied()).collect();
        assert_eq!(&written[..504],&target);
        assert_eq!(m.sent.iter().filter(|p|p[1]==1).count(),37);
    }
    #[test]
    fn stale_and_unknown_never_write() {
        let b = raw();
        let mut c = change(b);
        c.expected_state = "stale".into();
        let mut m = Mock {
            replies: replies(b).into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, c).unwrap_err().code, "stale_state");
        assert_eq!(m.sent.len(), 1);
        let mut c = change(b);
        c.key_id = "Q".into();
        let mut m = Mock {
            replies: replies(b).into(),
            sent: vec![],
        };
        assert_eq!(apply(&mut m, c).unwrap_err().code, "unsupported");
        assert_eq!(m.sent.len(), 1);
    }
    #[test]
    fn captured_three_layers_decode_504_readable_bytes() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../docs/evidence/kx-get41-fixture.json")).unwrap();
        for item in fixture["layers"].as_array().unwrap() {
            let layer=item["layer"].as_str().unwrap();
            let frames:Vec<Vec<u8>>=item["frames"].as_array().unwrap().iter().map(|p|hex::decode(p["hex"].as_str().unwrap()).unwrap()).collect();
            let expected:Vec<u8>=frames.iter().flat_map(|p|p[5..19].iter().copied()).collect();
            assert_eq!(frames.len(),36);assert_eq!(expected.len(),504);
            let mut t=Mock{replies:frames.into(),sent:vec![]};let raw=read(&mut t,layer).unwrap();
            assert_eq!(raw.as_slice(),expected.as_slice());assert_eq!(view("fixture",layer,raw).state_token.len(),1008);
            assert_eq!(t.sent.len(),1);assert_eq!(t.sent[0][4],layer_code(layer).unwrap());
            let payload=write_payload(raw);assert_eq!(&payload[..504],&expected);assert_eq!(&payload[504..],&[0,0,0,0,0,0,90,165]);
        }
    }
    #[test]
    fn mapping_errors_identify_pre_set_ack_and_readback() {
        let b=raw();let mut m=Mock{replies:VecDeque::new(),sent:vec![]};
        let e=apply(&mut m,change(b)).unwrap_err();assert!(e.message.contains("尚未发送SET"));assert_eq!(m.sent.len(),1);
        let mut m=Mock{replies:replies(b).into(),sent:vec![]};
        let e=apply(&mut m,change(b)).unwrap_err();assert!(e.message.contains("SET已发送，等待ACK失败"));assert_eq!(m.sent.len(),2);
        let mut r=replies(b);r.extend((0..37).map(|s|packet(1,37,s,14,&[]).to_vec()));
        let mut m=Mock{replies:r.into(),sent:vec![]};
        let e=apply(&mut m,change(b)).unwrap_err();assert!(e.message.contains("SET已发送，完整读回失败"));
    }
    #[test]
    fn readable_unknown_tail_changes_fail_full_readback() {
        let b=raw();let mut target=b;target[36..40].copy_from_slice(&[0,3,41,0]);target[450]^=1;
        let mut r=replies(b);r.extend((0..37).map(|s|packet(1,37,s,14,&[]).to_vec()));r.extend(replies(target));r.extend(replies(target));
        let mut m=Mock{replies:r.into(),sent:vec![]};assert_eq!(apply(&mut m,change(b)).unwrap_err().code,"readback_mismatch");
    }
    fn capture_layer(t:&mut impl Transport,layer:&str)->(Vec<Vec<u8>>,Option<String>) {
        let base=match layer_code(layer) {Ok(v)=>v,Err(e)=>return(vec![],Some(e.message))};
        if let Err(e)=t.send(&packet(0x41,1,0,base,&[])) {return(vec![],Some(e.message));}
        let deadline=Instant::now()+Duration::from_secs(10);
        let first=match kx_receive_until(t,0x41,0,deadline) {Ok(p)=>p,Err(e)=>return(vec![],Some(e.message))};
        let total=first[2];let mut frames=vec![first];
        if total==0||total>64 {return(frames,Some(format!("GET41包数超出1..64：{total}")));}
        for seq in 1..total {
            if Instant::now()>=deadline {return(frames,Some("GET41整层读取超过10秒".into()));}
            match kx_receive_until(t,0x41,seq,deadline) {
                Ok(p)=>{let same=p[2]==total;frames.push(p);if !same {return(frames,Some("GET41包数中途改变".into()));}},
                Err(e)=>return(frames,Some(e.message)),
            }
        }
        (frames,None)
    }
    #[test]
    #[ignore="Explicit read-only GET41 capture for three KX75 layers; no SET, no network"]
    fn hardware_kx75_readonly_keymap_layers_once() {
        let api=hidapi::HidApi::new().expect("HID enumeration");
        let d=api.device_list().find(|d|d.vendor_id()==0x3554&&d.product_id()==0xfa09&&d.usage_page()==0xff02&&d.usage()==2&&d.interface_number()==1).expect("precise KX75 receiver interface");
        let id=hex::encode(d.path().to_bytes());let q=crate::device_queue(&id);let _guard=q.lock().unwrap();
        let device=d.open_device(&api).expect("open receiver");crate::verify_kx75(&device).expect("KX75 identity");
        let mut t=KxTransport{device};let mut layers=Vec::new();let mut failure=None;
        for layer in ["Default","Fn","Fn2"] {
            let started=Instant::now();let (frames,error)=capture_layer(&mut t,layer);
            let rows:Vec<_>=frames.iter().map(|p|serde_json::json!({"hex":hex::encode(p),"total":p[2],"sequence":p[3],"yy":p[4],"bytes":p.len()})).collect();
            if let Some(first)=frames.first() {println!("GET41 layer={layer} total={} first_yy={} packets={} payload_bytes={} elapsed_ms={}",first[2],first[4],frames.len(),frames.len()*14,started.elapsed().as_millis());}
            layers.push(serde_json::json!({"layer":layer,"requestHex":hex::encode(packet(0x41,1,0,layer_code(layer).unwrap(),&[])),"elapsedMs":started.elapsed().as_millis(),"frames":rows,"error":error}));
            if error.is_some() {failure=error;break;}
        }
        let record=serde_json::json!({"capturedAtUnixMs":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),"source":"verified KX75 native read-only GET41; no SET, no HID path or serial","layers":layers});
        let output=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../artifacts/official-settings-20261024/kx-get41-readonly-frames.json");
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();std::fs::write(output,serde_json::to_vec_pretty(&record).unwrap()).unwrap();
        assert!(failure.is_none(),"{}",failure.unwrap_or_default());
    }
}
