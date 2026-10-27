//! KX75 built-in effects in the existing 128-byte performance block.
//! df() explicitly falls back to uae/hae for the KX75 RGB block.
use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Change {
    expected_state: String,
    mode: Option<u8>,
    brightness: Option<u8>,
    speed: Option<u8>,
    multi_color: Option<bool>,
    rgb: Option<[u8; 3]>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    device_id: String,
    model: &'static str,
    sampled_at: u64,
    state_token: String,
    mode: u8,
    brightness: Option<u8>,
    speed: Option<u8>,
    multi_color: Option<bool>,
    rgb: Option<[u8; 3]>,
    color_error: Option<String>,
    modes: Vec<Mode>,
    verified_on_hardware: bool,
    unsupported: Vec<&'static str>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Mode {
    id: u8,
    label: &'static str,
    brightness_editable: bool,
    speed_editable: bool,
    color_editable: bool,
}
fn modes() -> Vec<Mode> {
    [
        (0, "关闭"),
        (1, "常亮模式"),
        (2, "呼吸模式"),
        (3, "梦幻彩虹"),
        (4, "一触即发"),
        (5, "雨中漫步"),
        (7, "涟漪扩散"),
        (8, "繁星点点"),
        (10, "川流不息"),
        (11, "随波逐流"),
        (12, "如影随形"),
        (13, "正弦光波"),
        (15, "旋转风车"),
        (16, "七彩瀑布"),
        (17, "花开富贵"),
    ]
    .into_iter()
    .map(|(id, label)| Mode {
        id,
        label,
        brightness_editable: !matches!(id, 0 | 15 | 17),
        speed_editable: !matches!(id, 0 | 1 | 15 | 17),
        color_editable: !matches!(id, 0 | 3 | 15 | 16 | 17),
    })
    .collect()
}
// P5t and K5t order: 58 bytes of prefix, then two bytes per effect slot.
// Effect 15 is packed immediately after 13, with one reserved pair after 15.
fn offset(mode: u8) -> Option<usize> {
    match mode {
        1..=5 | 7..=8 | 10..=13 => Some(58 + (mode as usize - 1) * 2),
        15 => Some(84),
        16 => Some(88),
        17 => Some(90),
        _ => None,
    }
}
fn view(id: &str, b: [u8; 128]) -> Snapshot {
    let mode = b[10];
    let pair = (b[9] == 0)
        .then(|| offset(mode))
        .flatten()
        .map(|o| (b[o], b[o + 1]));
    Snapshot {
        device_id: id.into(),
        model: "MCHOSE KX75",
        sampled_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        state_token: hex::encode(b),
        mode,
        brightness: pair.and_then(|(v, _)| (1..=4).contains(&v).then_some(v)),
        speed: pair.and_then(|(_, v)| (1..=4).contains(&(v >> 4)).then_some(v >> 4)),
        multi_color: pair.and_then(|(_, v)| match v & 15 {
            0 => Some(false),
            7 => Some(true),
            _ => None,
        }),
        rgb: None,
        color_error: None,
        modes: modes(),
        verified_on_hardware: false,
        unsupported: vec![
            "逐键DIY、音乐与GIF灯效尚未实现",
        ],
    }
}
fn target(before: [u8; 128], c: &Change) -> Result<[u8; 128]> {
    if c.mode.is_none() && c.brightness.is_none() && c.speed.is_none() && c.multi_color.is_none() && c.rgb.is_none() {
        return Err(err("invalid_parameter", "请选择灯效设置"));
    }
    if before[9] != 0 && c.mode.is_none() {
        return Err(err(
            "unsupported",
            "当前不是已核实内置灯效类型，请先显式选择内置模式",
        ));
    }
    let mode = c.mode.unwrap_or(before[10]);
    let spec = modes()
        .into_iter()
        .find(|m| m.id == mode)
        .ok_or_else(|| err("unsupported", "未知或自定义灯效，请选择已核实的内置模式"))?;
    let mut b = before;
    if c.mode.is_some() {
        b[9] = 0;
        b[10] = mode;
    }
    if let Some(v) = c.brightness {
        if !spec.brightness_editable || !(1..=4).contains(&v) {
            return Err(err(
                "invalid_parameter",
                "该模式不支持亮度设置，或亮度不在1至4",
            ));
        }
        b[offset(mode).unwrap()] = v;
    }
    if let Some(v) = c.speed {
        if !spec.speed_editable || !(1..=4).contains(&v) {
            return Err(err(
                "invalid_parameter",
                "该模式不支持速度设置，或速度不在1至4",
            ));
        }
        let o = offset(mode).unwrap() + 1;
        b[o] = (b[o] & 0x0f) | (v << 4);
    }
    if let Some(v) = c.multi_color {
        if !spec.color_editable {
            return Err(err("unsupported", "该模式不支持单色/多色切换"));
        }
        let o = offset(mode).unwrap() + 1;
        b[o] = (b[o] & 0xf0) | if v { 7 } else { 0 };
    }
    if c.rgb.is_some() && (!spec.color_editable || b[offset(mode).unwrap()+1]&15 != 0) {
        return Err(err("unsupported","RGB颜色仅在支持单色的内置灯效中编辑"));
    }
    Ok(b)
}
fn read_colors(t: &mut impl Transport) -> Result<[u8;490]> {
    t.send(&kx_packet(0x49,1,0,&[]))?;
    let deadline=Instant::now()+Duration::from_secs(3);
    let mut b=[0u8;490];
    for seq in 0..35 {
        if Instant::now()>=deadline {return Err(err("timeout","KX75颜色块读取超过3秒"));}
        let p=kx_receive(t,0x49,seq)?;
        if p[2]!=35 {return Err(err("invalid_response",format!("KX75颜色块包数未匹配：total={} seq={} YY={}，请保留捕获核对",p[2],p[3],p[4])));}
        b[seq as usize*14..seq as usize*14+14].copy_from_slice(&p[5..19]);
    }
    Ok(b)
}
fn color_packets(colors:[u8;490])->Vec<[u8;20]> {
    // Official mae(false)/ot: 490-byte configuration + 14 zero command bytes + 14 control bytes.
    // The final 5A A5 marker belongs to the write transport and is absent from GET49 data.
    let mut payload=[0u8;518];
    payload[..490].copy_from_slice(&colors);
    payload[506..508].copy_from_slice(&[90,165]);
    payload.chunks(14).enumerate().map(|(seq,chunk)| {
        let mut p=kx_packet(9,37,seq as u8,chunk);
        p[4]=if seq==36 {14-chunk.iter().rev().take_while(|b|**b==0).count() as u8} else {14};
        p[19]=p[..19].iter().fold(0u8,|a,b|a.wrapping_add(*b));p
    }).collect()
}
fn lighting_view(id:&str,b:[u8;128],colors:Result<[u8;490]>) -> Snapshot {
    let mut s=view(id,b);
    match colors {
        Ok(colors)=>{
            s.state_token.push_str(&hex::encode(colors));
            if b[9]==0 && modes().iter().any(|m|m.id==b[10]&&m.color_editable) {
                let o=b[10] as usize*21;
                s.rgb=Some(colors[o..o+3].try_into().unwrap());
            }
        }
        Err(e)=>s.color_error=Some(format!("颜色读取失败：{}",e.message)),
    }
    s
}
fn apply_state(t: &mut impl Transport, c: Change) -> Result<([u8;128],Option<[u8;490]>)> {
    let before = kx_read(t).map_err(|e|kx_stage(e,"应用前读取灯效性能失败（尚未发送SET）"))?;
    let colors=if c.expected_state.len()==(128+490)*2 {Some(read_colors(t).map_err(|e|kx_stage(e,"应用前读取颜色失败（尚未发送SET）"))?)} else {None};
    let mut token=hex::encode(before);
    if let Some(colors)=colors {token.push_str(&hex::encode(colors));}
    if token!=c.expected_state {return Err(err("stale_state","KX75性能或颜色块已变化，请刷新预览"));}
    let wanted = target(before, &c)?;
    let mut wanted_colors=colors;
    if let Some(rgb)=c.rgb {
        let colors=wanted_colors.as_mut().ok_or_else(||err("unsupported","尚未读取完整颜色块，请刷新后再编辑RGB"))?;
        let o=wanted[10] as usize*21;
        colors[o..o+3].copy_from_slice(&rgb);
    }
    if wanted_colors!=colors {
        let colors=wanted_colors.unwrap();
        for (seq,p) in color_packets(colors).into_iter().enumerate() {
            t.send(&p).map_err(|e|kx_stage(e,"颜色SET发送失败，可能部分写入，请刷新"))?;
            kx_receive(t,9,seq as u8).map_err(|e|kx_stage(e,"颜色SET已发送，等待ACK失败，请刷新"))?;
        }
        if read_colors(t).map_err(|e|kx_stage(e,"颜色SET已发送，完整读回失败，请刷新"))?!=colors {return Err(err("readback_mismatch","KX75完整颜色块读回不一致，可能已写入，请刷新"));}
    }
    if before!=wanted {
        for (seq, chunk) in wanted.chunks(14).enumerate() {
            t.send(&kx_packet(4, 10, seq as u8, chunk)).map_err(|e|kx_stage(e,"灯效性能SET发送失败，可能部分写入，请刷新"))?;
            kx_receive(t, 4, seq as u8).map_err(|e|kx_stage(e,"灯效性能SET已发送，等待ACK失败，请刷新"))?;
        }
        let mut verified=false;
        for _ in 0..2 {if kx_read(t).map_err(|e|kx_stage(e,"灯效性能SET已发送，完整读回失败，请刷新"))?==wanted {verified=true;break;}}
        if !verified {return Err(err("readback_mismatch","KX75完整性能块读回不一致，可能已写入，请刷新"));}
    }
    if before==wanted && wanted_colors!=colors && kx_read(t).map_err(|e|kx_stage(e,"颜色SET已发送，最终性能核对失败，请刷新"))?!=wanted {return Err(err("readback_mismatch","KX75颜色应用后性能块发生变化，请刷新"));}
    if let Some(final_colors)=wanted_colors {if read_colors(t).map_err(|e|kx_stage(e,if before!=wanted||wanted_colors!=colors {"设置SET已发送，最终颜色核对失败，请刷新"} else {"核对颜色失败（未发送SET）"}))?!=final_colors {return Err(err("readback_mismatch","KX75最终完整颜色块发生变化，请刷新"));}}
    Ok((wanted,wanted_colors))
}
#[cfg(test)]
fn apply(t:&mut impl Transport,c:Change)->Result<[u8;128]> {apply_state(t,c).map(|s|s.0)}
pub(crate) fn read_sync(id: &str) -> Result<Snapshot> {
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = super::kx_mapping::open(id)?;
    let b=kx_read(&mut t).map_err(|e|kx_stage(e,"读取灯效性能失败（未发送设置写入）"))?;
    Ok(lighting_view(id,b,read_colors(&mut t)))
}
pub(crate) fn apply_sync(id: &str, c: Change) -> Result<Snapshot> {
    let q = crate::device_queue(id);
    let _guard = q.lock().map_err(|_| err("queue", "设备队列不可用"))?;
    let mut t = super::kx_mapping::open(id)?;
    let (b,colors)=apply_state(&mut t,c)?;
    Ok(match colors {Some(colors)=>lighting_view(id,b,Ok(colors)),None=>lighting_view(id,b,Err(err("unsupported","本次预览未包含颜色块，请重新读取")))})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Mock {
        sent: Vec<Vec<u8>>,
        replies: VecDeque<Vec<u8>>,
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
    fn raw() -> [u8; 128] {
        let mut b = [0xab; 128];
        b[9] = 0;
        b[10] = 2;
        b[24] = 4;
        b[60] = 4;
        b[61] = 0x37;
        b
    }
    fn change(b: [u8; 128]) -> Change {
        Change {
            expected_state: hex::encode(b),
            mode: None,
            brightness: Some(2),
            speed: Some(4),
            multi_color: Some(false),
            rgb: None,
        }
    }
    fn replies(b: [u8; 128]) -> Vec<Vec<u8>> {
        b.chunks(14)
            .enumerate()
            .map(|(s, c)| kx_packet(0x44, 10, s as u8, c).to_vec())
            .collect()
    }
    #[test]
    fn lighting_preserves_every_non_target_byte() {
        let b = raw();
        let w = target(b, &change(b)).unwrap();
        assert_eq!(&w[60..62], &[2, 0x40]);
        for i in 0..128 {
            if !(60..62).contains(&i) {
                assert_eq!(b[i], w[i], "byte {i}");
            }
        }
        let mut c = change(b);
        c.multi_color = None;
        assert_eq!(target(b, &c).unwrap()[61], 0x47);
        c.speed = None;
        c.multi_color = Some(false);
        assert_eq!(target(b, &c).unwrap()[61], 0x30);
    }
    #[test]
    fn mode_offsets_and_capabilities_do_not_borrow_other_models() {
        assert_eq!(offset(13), Some(82));
        assert_eq!(offset(15), Some(84));
        assert_eq!(offset(16), Some(88));
        for id in [6, 9, 14, 19, 21, 255] {
            let mut c = change(raw());
            c.mode = Some(id);
            assert!(target(raw(), &c).is_err());
        }
        let mut c = change(raw());
        c.mode = Some(1);
        assert!(target(raw(), &c).is_err());
        c.speed = None;
        c.mode = Some(3);
        assert!(target(raw(), &c).is_err());
        c.multi_color = None;
        assert!(target(raw(), &c).is_ok());
        c.brightness = Some(5);
        assert!(target(raw(), &c).is_err());
    }
    #[test]
    fn unknown_mode_is_visible_without_fallback() {
        let mut b = raw();
        b[10] = 222;
        let s = view("fixture", b);
        assert_eq!(s.mode, 222);
        assert_eq!(s.brightness, None);
        assert!(target(b, &change(b)).is_err());
        b[10] = 2;
        b[9] = 1;
        assert_eq!(view("fixture", b).brightness, None);
        assert!(target(b, &change(b)).is_err());
        let mut c = change(b);
        c.mode = Some(2);
        assert_eq!(target(b, &c).unwrap()[9], 0);
    }
    #[test]
    fn full_transaction_requires_full_readback() {
        let b = raw();
        let w = target(b, &change(b)).unwrap();
        let mut r = replies(b);
        r.extend((0..10).map(|s| kx_packet(4, 10, s, &[]).to_vec()));
        r.extend(replies(w));
        let mut m = Mock {
            sent: vec![],
            replies: r.into(),
        };
        assert_eq!(apply(&mut m, change(b)).unwrap(), w);
        assert_eq!(m.sent.len(), 12);
        assert_eq!(&m.sent[10][..7], &[19, 4, 10, 9, 2, w[126], w[127]]);
    }
    #[test]
    fn stale_snapshot_sends_no_set_and_ack_loss_stops() {
        let b = raw();
        let mut c = change(b);
        c.expected_state = "stale".into();
        let mut m = Mock {
            sent: vec![],
            replies: replies(b).into(),
        };
        assert_eq!(apply(&mut m, c).unwrap_err().code, "stale_state");
        assert_eq!(m.sent.len(), 1);
        let mut m = Mock {
            sent: vec![],
            replies: replies(b).into(),
        };
        assert!(apply(&mut m, change(b)).is_err());
        assert_eq!(m.sent.len(), 2);
    }
    #[test]
    fn unrelated_readback_changes_fail() {
        let b = raw();
        let mut w = target(b, &change(b)).unwrap();
        w[100] ^= 1;
        let mut r = replies(b);
        r.extend((0..10).map(|s| kx_packet(4, 10, s, &[]).to_vec()));
        r.extend(replies(w));
        r.extend(replies(w));
        let mut m = Mock {
            sent: vec![],
            replies: r.into(),
        };
        assert_eq!(
            apply(&mut m, change(b)).unwrap_err().code,
            "readback_mismatch"
        );
    }
    fn colors()->[u8;490] {[0x5c;490]}
    fn color_replies(b:[u8;490])->Vec<Vec<u8>> {b.chunks(14).enumerate().map(|(s,c)|kx_packet(0x49,35,s as u8,c).to_vec()).collect()}
    #[test]
    fn rgb_transaction_changes_only_target_color_and_preserves_tail() {
        let mut b=raw();b[61]=0x30;let colors=colors();let mut wanted=colors;
        wanted[42..45].copy_from_slice(&[218,203,218]);
        let mut c=change(b);c.brightness=None;c.speed=None;c.multi_color=None;c.rgb=Some([218,203,218]);c.expected_state.push_str(&hex::encode(colors));
        let mut r=replies(b);r.extend(color_replies(colors));r.extend((0..37).map(|s|kx_packet(9,37,s,&[]).to_vec()));r.extend(color_replies(wanted));r.extend(replies(b));r.extend(color_replies(wanted));
        let mut m=Mock{sent:vec![],replies:r.into()};let (_,actual)=apply_state(&mut m,c).unwrap();assert_eq!(actual,Some(wanted));
        assert!(!m.sent.iter().any(|p|p[1]==4));
        let written:Vec<_>=m.sent.iter().filter(|p|p[1]==9).collect();assert_eq!(written.len(),37);
        let actual:Vec<_>=written.iter().flat_map(|p|p[5..19].iter().copied()).collect();
        assert_eq!(&actual[..490],&wanted);
        assert_eq!(&actual[490..506],&[0;16]);assert_eq!(&actual[506..508],&[90,165]);assert_eq!(&actual[508..518],&[0;10]);
        assert_eq!(written.last().unwrap()[4],4);
        for i in 0..490 {if !(42..45).contains(&i) {assert_eq!(colors[i],wanted[i]);}}
    }
    #[test]
    fn color_failure_keeps_performance_and_reports_error() {
        let s=lighting_view("fixture",raw(),Err(err("timeout","fixture timeout")));
        assert_eq!(s.mode,2);assert_eq!(s.brightness,Some(4));assert!(s.rgb.is_none());assert!(s.color_error.is_some());assert_eq!(s.state_token.len(),256);
        let mut c=change(raw());c.rgb=Some([1,2,3]);assert!(target(raw(),&c).is_ok());
        let mut b=raw();b[61]=0x37;c.multi_color=None;assert!(target(b,&c).is_err());
    }
    #[test]
    #[ignore = "Explicit GET44/GET49 KX75 read-only diagnostic; no SET, no network"]
    fn hardware_kx75_readonly_performance_and_color_once() {
        let api=hidapi::HidApi::new().expect("HID enumeration");
        let d=api.device_list().find(|d|d.vendor_id()==0x3554&&d.product_id()==0xfa09&&d.usage_page()==0xff02&&d.usage()==2&&d.interface_number()==1).expect("precise KX75 receiver interface");
        let id=hex::encode(d.path().to_bytes());let q=crate::device_queue(&id);let _guard=q.lock().unwrap();
        let device=d.open_device(&api).expect("open receiver");crate::verify_kx75(&device).expect("KX75 signature");
        let mut t=KxTransport{device};let started=Instant::now();
        let perf=kx_read(&mut t);println!("GET44 bytes={} elapsed_ms={}",perf.as_ref().map(|b|b.len()).unwrap_or(0),started.elapsed().as_millis());
        if let Ok(b)=perf {println!("GET44 light_type={} mode={} rate_raw={} sleep_raw={} latency_raw={} mac_raw={} win_raw={}",b[9],b[10],b[1],b[24],b[3],b[27],b[15]);}
        let color_started=Instant::now();
        let (frames,error)=capture_color_frames(&mut t);
        let rows:Vec<_>=frames.iter().map(|p|serde_json::json!({"total":p[2],"sequence":p[3],"yy":p[4],"bytes":p.len(),"hex":hex::encode(p)})).collect();
        let record=serde_json::json!({"capturedAtUnixMs":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),"source":"native HID read-only GET49 after verified KX75 identity","requestHex":hex::encode(kx_packet(0x49,1,0,&[])),"elapsedMs":color_started.elapsed().as_millis(),"payloadBytes":frames.len()*14,"frames":rows,"error":error});
        let output=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../artifacts/official-settings-20261023/kx-get49-readonly-frames.json");
        std::fs::create_dir_all(output.parent().unwrap()).expect("create evidence folder");
        std::fs::write(&output,serde_json::to_vec_pretty(&record).unwrap()).expect("save sanitized frames");
        if let Some(first)=frames.first() {println!("GET49 first total={} seq={} YY={} frame_bytes={} payload_bytes={} elapsed_ms={}",first[2],first[3],first[4],first.len(),frames.len()*14,color_started.elapsed().as_millis());}
        println!("GET49 packets={} complete={} evidence=artifacts/official-settings-20261023/kx-get49-readonly-frames.json",frames.len(),error.is_none());
        assert!(error.is_none(),"{}",error.unwrap_or_default());
    }
    // A diagnostic collector follows the device's reported count, without treating it as a write contract.
    fn capture_color_frames(t:&mut impl Transport)->(Vec<Vec<u8>>,Option<String>) {
        let mut frames=Vec::new();
        if let Err(e)=t.send(&kx_packet(0x49,1,0,&[])) {return(frames,Some(e.message));}
        let deadline=Instant::now()+Duration::from_millis(3500);
        let first=match kx_receive(t,0x49,0) {Ok(p)=>p,Err(e)=>return(frames,Some(e.message))};
        let total=first[2];frames.push(first);
        if total==0||total>64 {return(frames,Some(format!("颜色诊断包数超出1..64：{total}")));}
        for seq in 1..total {
            if Instant::now()>=deadline {return(frames,Some("颜色诊断超过3500ms".into()));}
            match kx_receive(t,0x49,seq) {
                Ok(p)=>{let same_total=p[2]==total;frames.push(p);if !same_total {return(frames,Some("颜色包total中途改变".into()));}}
                Err(e)=>return(frames,Some(e.message)),
            }
        }
        (frames,None)
    }
    #[test]
    fn diagnostic_collects_reported_packet_count_without_set() {
        let r=(0..35).map(|seq|kx_packet(0x49,35,seq,&[0x5a;14]).to_vec()).collect();
        let mut t=Mock{sent:vec![],replies:r};let (frames,error)=capture_color_frames(&mut t);
        assert!(error.is_none());assert_eq!(frames.len(),35);assert_eq!(t.sent.len(),1);assert_eq!(t.sent[0][1],0x49);
    }
    #[test]
    fn captured_get49_decodes_actual_490_bytes_without_write_marker() {
        // Sanitized read-only capture, 2026-10-23; configuration colors only, no HID path or serial.
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../docs/evidence/kx-get49-fixture.json")).unwrap();
        let frames:VecDeque<Vec<u8>>=fixture["frames"].as_array().unwrap().iter().map(|p|hex::decode(p["hex"].as_str().unwrap()).unwrap()).collect();
        let expected:Vec<u8>=frames.iter().flat_map(|p|p[5..19].iter().copied()).collect();
        assert_eq!(expected.len(),490);
        let mut t=Mock{sent:vec![],replies:frames};let colors=read_colors(&mut t).unwrap();assert_eq!(colors.as_slice(),expected.as_slice());
        let packets=color_packets(colors);assert_eq!(packets.len(),37);
        assert_eq!(&packets[35][5..19],&[0;14]);assert_eq!(&packets[36][5..9],&[0,0,90,165]);
        assert!(t.sent.iter().all(|p|p[1]==0x49));
    }
}
