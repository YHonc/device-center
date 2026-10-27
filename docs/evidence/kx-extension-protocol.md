# KX75 扩展协议证据

日期：2026-10-23。依据本地官网静态快照 `references/purify.es-CxLZ1Htw.js`。本轮仅源码核对和离线 mock；没有硬件 IO。

## 内置灯效

型号分派：`:54` 的 `df` 将 KX75 分派到 `$5t`；`:4334` 的导出是 `{getPerformanceParser:P5t,keyList:o6,performanceCommonParser:K5t,settingCommonParser:H5t}`。`:4327` 的 `A8.KX75` 使用 `otherObj:wQt`，不是 K99/G98 的默认块。

P5t 读取、K5t 写入共有128字节性能块。前缀逐项累加确认 `lightType` 位于9、`lightMode`位于10，亮度字段从58起。`space0`六字节只在有线解析出现，无线块偏移不含它。

| 模式 | 亮度偏移 | 速度/多色偏移 | 可调整 |
|---|---:|---:|---|
| 0 关闭 | 无 | 无 | 模式 |
| 1 常亮模式 | 58 | 59 | 亮度、单/多色 |
| 2 呼吸模式 | 60 | 61 | 亮度、速度、单/多色 |
| 3 梦幻彩虹 | 62 | 63 | 亮度、速度 |
| 4 一触即发 | 64 | 65 | 亮度、速度、单/多色 |
| 5 雨中漫步 | 66 | 67 | 亮度、速度、单/多色 |
| 7 涟漪扩散 | 70 | 71 | 亮度、速度、单/多色 |
| 8 繁星点点 | 72 | 73 | 亮度、速度、单/多色 |
| 10 川流不息 | 76 | 77 | 亮度、速度、单/多色 |
| 11 随波逐流 | 78 | 79 | 亮度、速度、单/多色 |
| 12 如影随形 | 80 | 81 | 亮度、速度、单/多色 |
| 13 正弦光波 | 82 | 83 | 亮度、速度、单/多色 |
| 15 旋转风车 | 84 | 85 | 模式 |
| 16 七彩瀑布 | 88 | 89 | 亮度、速度 |
| 17 花开富贵 | 90 | 91 | 模式 |

上述是官网灯效组件 `const Tt=[{name:s("keyboard.alwaysOn")...` 的 KX75 分支与 P5t/K5t字段交集，包含关闭共15项。官网6只开放G75_V2；14/19仅用于列出的G98_Pro/G98_V2/G87_V2/UT98/Z75/Z75S；21为独立DIY区，不开放。KX75 不在这些例外型号中。

同组件滑块 `min:1,max:...G75_Pro?9:4` 确认 KX75亮度1至4；速度 `min:[G98_Pro,...].includes(g)?0:1,max:[...].includes(g)?5:4` 确认1至4。没有借用全局灯光同步的百分比范围。

K5t明确先写`speedN:bit4`再写`multiColorN:bit4`。`:3881`附近的`KY`对bit4依次拼二进制字符串，满8位后转整数。因此速度在高4位、多色在低4位。官网`He`写多色7、单色0；`wQt`默认速度3、多色7形成`0x37`。单项写入保留另一半字节，不校正未提交的未知字段。

接口：`read_keyboard_lighting({deviceId})`；`apply_keyboard_lighting({deviceId,change:{expectedState,mode?,brightness?,speed?,multiColor?}})`。mode仅显式提交时设置lightType=0；未提交时保留。未知模式原值直接显示且属性为null，不填默认模式；未知lightType也将属性显示为null，要求显式选择内置模式后才能编辑。用户可以显式选择已核实内置模式。相同模式的未知参数保留原始值，仅修改用户提交的字段。

事务沿用设备同锁、3554/FA09/FF02/usage2/interface1及每次KX75六字节型号probe。GET44读取128字节后比较完整expectedState；SET04按14字节分10包，最后2字节；每包匹配ACK，最后最多两轮完整128字节读回。无变化不SET，失败不自动重写。校验与回复包数的实机边界沿用 SETTINGS.md。

## RGB颜色区

`:3881`的`df`完整返回分支明确包含`lightColorCommonParser:s||hae,getLightColorParser:o||uae`。KX75 `$5t`未导出专用解析器时，官网主动使用`uae/hae`默认解析器。这是型号分派后的真实回退，不是根据相似型号推测。本轮早期只看专用导出的结论已纠正。

`uae(false)`逐项定义0至13共14槽，每槽21字节，后跟196字节empty14，共490字节。RGB字段在`mode*21`起的3字节，其后18字节不变。GET49实机为35个14字节载荷，共490字节，不含写入控制尾部。

`mae(false)`返回`[{name:"command",type:"uint8"},...hae,{name:"empty15",...},{name:"empty16",...}]`。官网灯效组件`ot`对无线KX75明确传`empty15:[0,0,0,0,0,0,0,0,0,0,0,0,0,0]`、`empty16:[0,0,90,165,0,0,0,0,0,0,0,0,0,0]`。因此SET09传输区为490配置+28控制=518字节，37个14字节载荷；90/165仅位于写传输区506/507，不是GET配置的尾标记。SET09的`pae`只有尾包使用`14-trailingZeros`，全零则0；此固定控制尾部的YY为4。实现从真实490字节配置构造，仅改目标RGB三字节，再追加这些已核实运输字段。

灯效Snapshot新增`rgb:[u8;3]|null`和`colorError:string|null`；Change新增`rgb?:[u8;3]`。颜色成功读取时stateToken覆盖性能128+颜色490字节，应用先重读两块并全量CAS，只修改目标模式RGB三字节；颜色先写后完整490字节读回，再修改性能块并最终核对颜色。单色且该模式允许调色才接受RGB。颜色读取失败仍返回性能灯效及明确colorError，保留性能块token，不允许缺颜色快照的RGB写入。逐键DIY、音乐、GIF尚未开放。

## 官方动作目录与性能字段

`:4334`的键盘设置使用`DF`基础104项、`t3`鼠标7项、`n3`媒体7项、`i3`扩展48项。它们的`val`均直接进入选中按键，并按当前层写入`setKeySetting`。`kx_mapping.rs`完整移植这些四字节值，Action增加`category:base/mouse/media/extended`。保留已有key:A至key:0、taskManager和security ID。新增disabled为全零。Fn/Fn2层拒绝分配Fn/Fn2，与官网`Nt`门控一致。

旧实现只识别字母数字和两预设，导致Esc、F键、修饰键、媒体、全零Fn2值被标只读；本轮已修正。KX75 `_vt`额外的原厂Fn值按明确字节识别为可替换原值，但不加入可分配目录，不生成新的固件原生功能。其余未知原值仍保留只读。

`:4644`的`performanceSetting`与P5t/K5t共同确认：`rateVal[1]`为1/2/3对应250/500/1000Hz；`latencySwitch[3]`开0/关2；`macSwitch[27]`在`l.KX75.on`为2、关0；`winKeySwitch[15]`开1/关0；休眠仍为[24]、分钟乘2。官网开启Mac同时清Win锁并禁用控件；不是只切一个UI布尔值。共享settings模块负责接入这些字段。

枚举门控核对：lib.rs的`kx_candidate`及kx_mapping::open均按3554/FA09/FF02/usage2/interface1匹配，没有product字符串白名单，接收器名称后缀-KX75不会被当前后端过滤。官网`XS`使用识别后的realName/productName分派；URL上的-KX75不能直接当作原生product字段。

## 任意组合键与宏

`:4320`的`kSt`组合组件生成鼠标`buttonValue:0x13sshh`；`N1e={Ctrl:1,Shift:2,Alt:4,Win:8}`不能直接当键盘四字节定义。`:4334`的键盘`Nt`读取`selectedItem`的`{name,val,info,type}`，没有该buttonValue的转换链。现有两项官网预设不等于任意组合键能力。

键盘宏分配入口生成`[3,{0:4,1:2,2:1}[type],1,index]`。`:3881`的`Rpe`确有GET43、SET03、跨512字节区YY；但`getMacroParser`只暴露单个uint32和长度字段，目前完整已有宏区读取、可保留字节、索引/容量/名称绑定契约未闭合。设备宏写入和用宏模拟组合键均未实现。不能把本地录制草稿声称为设备宏成功。

## 离线回归

`kx_extensions.rs`的mock覆盖目标字节/半字节保留、准确效果偏移、其他型号模式拒绝、未知模式显示、完整写包/末包、CAS过期不SET、中途ACK丢失立即停止、非目标字节读回改变拒绝。测试由主代理统一运行；完整硬件写入验收留到最后。

## 本轮只读探针与身份查询差异

主代理反馈本轮Rust默认测试76通过、0失败、3忽略。单独运行`hardware_kx75_readonly_performance_and_color_once`时，在`verify_kx75`等待身份应答500ms超时，尚未发送GET44和GET49。该失败不是颜色读取失败，也不能证明颜色分包或CRC错误；这些阶段未到达。官网稍早已识别KX75并显示2.4G与100%，不能以旧界面值推断失败瞬间设备仍醒着。

身份链对照：官网`Yat→Vpe→nQ→sendCommand("get","getPassWord",{},{retryTime:2})`。`pJe`识别无线名，使`dJe.isWired=false`，`readCommand`走`device.sendReport(currentReportId,a)`。`currentReportId=0x13`；`iJe`剥原始命令首报告ID，发送19字节data，因此对应原生HID的20字节Output write，现有请求方式与长度匹配。Feature仅为`isWired=true`分支，不能用于当前接收器。

官网`Rpe`请求为`13 05 01`、16个零、校验`19`，本地verify请求一致。官网`handleCommandResponse`从不含报告ID的inputreport数据去4字节header后拼password，对应原生有报告ID回复的[5..11]；`Bat.KX75=0x0300000001c7`与本地型号签名一致。

已发现有证据的时限差异：官网此次身份调用`retryTime:2`使`waitForCompleteResponse`使用`200+retryTime*1000=2200ms`总等待；本地当时`read_kx_reply`固定500ms。官网该读取方法没有在此处重发GET。建议只给身份探测使用2200ms单次有界等待，由共享lib集成；等待放宽本身尚不能称为实机故障修复。没有新的设备状态证据前，不重复运行探针。

用户唤醒后的新证据：身份回复`130501000a0300000001c70100000000000000ef`成功；GET44完整128字节在44ms返回，lightType=0、mode=7、rateVal=3、sleepTimeVal=4、latencySwitch=2、macSwitch=0、winKeySwitch=0。GET49在52ms因实际包数不等于37被当前解析拒绝；不能把SET09推导出的37包直接当GET49契约。只读诊断已改为按设备报告total收集，在64包和3500ms内保存每帧头与完整hex，待主代理执行。该诊断不含任何SET。生产颜色读取在真实捕获之前保持明确错误，不填补颜色默认值。

随后有界GET49成功：首包total=35、sequence=0、YY=14、原生帧20字节；完整35包载荷490字节，耗时约147ms（证据文件记录146ms）。原始脱敏帧位于`artifacts/official-settings-20261023/kx-get49-readonly-frames.json`，精选副本`docs/evidence/kx-get49-fixture.json`用于生产读取回归。生产read_colors据此改为35包490字节；mock验证GET35包与SET37包的差异，并核对仅目标三字节变化及精确运输尾部。此证据证明只读配置获取，尚无任何SET或写后设备行为验收。

## 2026-10-24 键表读写长度与错误阶段修复

用户实窗中按键页报“键表分包数量不符合512字节契约”；性能页在保留此前成功读数时，纯重新读取却显示“写入可能未完成”。两者分别来自读写长度混淆，以及底层超时错误不区分操作阶段。

只读GET41捕获一次先在500ms首包截止时失败。官网`keyBoard.Ke`确证GET41层YY为Default=00、Fn=10、Fn2=20，与本地请求一致；官网`waitForCompleteResponse`此处默认整层10秒。实现改为单GET、整层共享10秒绝对截止，传输层尊重剩余等待；每次最多跳32个无关报告，ACK仍500ms。新的三层捕获均成功：Default/Fn/Fn2各36包、504字节，YY分别14/30/46，约147/149/148ms。首包超时的环境原因仍未独立确证，不能因后一次成功断言延长等待解决了全部连接问题。

原始脱敏证据`artifacts/official-settings-20261024/kx-get41-readonly-frames.json`，精选回归副本`docs/evidence/kx-get41-fixture.json`。`getKeySetting`有线请求长度`F8 01`也为504。`fae(false)`仅解析o6的96键×4=384字节，但设备额外返回120字节；本地将完整504字节全部保留，不像官网只用解析出的键槽重建。

写入链为`A8.KX75.commonObj=$vt`，其中`space15:new Array(126).fill(0),fix:[90,165]`；`H5t`按96槽、space15、fix序列化。官网每次`setKeySetting`以该commonObj固定尾构造，再覆盖按键字段，没有给该尾单独读取或编辑入口。主审据此将未返回的6个space字节及2个fix字节界定为厂商写请求契约，采用`[0,0,0,0,0,0,90,165]`补足504至512。前504中的120个额外字节仍逐字节保留，只有用户目标键4字节变化。SET01共37包，最后8字节尾使YY为层base+14；CAS与完整读回只比较504字节，未返回的8字节不声称得到实机读回验证。真实SET与实际按键行为仍未执行。

底层KxTransport超时现为中性“KX75响应超时”。只读入口明确未发送设置；应用前读取失败标“尚未发送SET”；发送调用失败标可能部分写入；已发送SET后的ACK或读回失败分别说明阶段。前端保留原快照并要求重新读取，不能自行给所有读取错误附加“可能写入”。新增mock覆盖三阶段错误、36包读取与37包写请求、固定尾与末包YY、未知可读尾字节保留及读回异常；真实三层捕获验证只读解析。

## 官网最新浏览器响应复核

主代理于2026-10-23通过Codex内置浏览器加载官网，将CDP `Network.getResponseBody`响应保存在`references/web/20261023-mchose/`。本代理仅对保存响应做离线处理。

新旧`purify.es-CxLZ1Htw.js`逐文件SHA256相同：`b64fdf6d0f85aa9548c8d7ddb43a7f1b39e4d662762d5e3f359e69a600fbdfae`。因此上述主协议证据在这次加载中仍然一致。

`keyboardConfig.json`、`keyboardPreset.json`含`__compressBase64__`，Base64解码后为ZIP，均只有`data.json`。离线解压并格式化到同目录`keyboardConfig.decoded.json`、`keyboardPreset.decoded.json`。前者20项，使用3837型号或Ace/G98 V3/God60/K87 V3等型号名；后者80项为3837/41E4的型号/固件版本到预设URL的映射。两个解压结果没有KX75、3554、FA09。不能用其他型号的新增配置推定KX75能力；本轮RGB链依据同版主JS中的df默认分支闭合。
