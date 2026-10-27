# ATK F1 DPI 与宏扩展依据

日期：2026-10-23。实现：`src-tauri/src/settings/atk_extensions.rs`。本记录只使用已保留的官网静态快照，没有连接设备或访问网络。

## 身份与接口

继续使用现有 `atk()` 的精确 `373B/101A/FF02/usage2/interface1` 接口白名单和同设备队列。接收器产品名不决定鼠标传感器。每次事务先发送 `Dr.get/GetMouseCIDMID` 命令16，从原生回复数据[6]/[7]取得CID/MID，再查白名单。

`references/filter-BRWy5Cag.js:1` 的 `CompxFiltersCustom` 注册证实：2,36/37 对应F1 PRO/PRO MAX、PAW3950；2,42/45/50/56 对应F1 Ultimate/Ultra/EXTREME/LEVIATAN、PAW3950Ultra；2,1/2/104、2,3/4/105对应Dragonfly F1 PRO/PRO MAX、PAW3395。其他CID/MID返回unsupported，不套用官网未知型号默认PAW3950Ultra。

## DPI

同快照的 `i.Address`、`Gt`、`Mr`、`U._getMouseDpiValues/setMouseDpiValues` 证实地址12/20/28/36，每块8字节、两档；每档 `[xLow,yLow,ex,85-xLow-yLow-ex]`。扩展位低半字节属于X，高半字节属于Y；每轴扩展位2/3为高两位，位0为倍增。

`ha/Ma` 是PAW3950及PAW3395分支：`(raw10bit+1)*50*(倍增?2:1)`。`la/pa` 是PAW3950Ultra分支：没有位1时 `(raw10bit+1)*10`，有位1时 `raw10bit*50+10050`，再应用位0倍增。`V=10000/Q=30000/He=10050/ne=3`是同文件常量。

`references/mouse-skins-TjQFnzLE.js:1` 的传感器规格：PAW3950Ultra为10..42000，至10000步长10、至30000步长50、以上步长100；PAW3950为50..36000，但30000以上编码要求步长100；PAW3395为100..30000、步长50。任意输入指允许用户按支持步长编辑数值，不接受801等无法准确表达的值，不静默取整。

`read_atk_dpi` 返回现有活动档位和每档XY数值、分段步长。`apply_atk_dpi`仅修改已有档位的一对8字节块，保留相邻档位；不新增档位。CAS覆盖原10字节设置块及全部32字节DPI区，并包含CID/MID。当前档位需校验正确且解码再编码逐字节一致，否则保持只读；三个完整读回之一与目标一致才成功。

## 宏内容与绑定

`filter-BRWy5Cag.js:1` 的 `U.getMacro/setMacro`、`Le`、`Pe` 证实12个可用槽，基址768，每槽跨度384；动作计数在+31，动作从+32开始，单个5字节，最多70动作，动作区最后是 `85-sum(count+actions)`。头31字节是 `[8,2*8,255*22]`。全0或全255槽作为空白；已有非空头/校验/动作不匹配时拒绝覆盖。

动作编码 `[state|type,value1,value2,delayHigh,delayLow]`，Down位0x80、Up位0x40；type0修饰键，1普通键，4鼠标键。普通键当前开放HID4..115；修饰键224..231映射1<<index；鼠标键1/2/4/8/16。value2固定0；延迟16位大端，非末动作至少1ms、末动作允许0ms，全部按下/抬起需平衡。延迟属于本动作之后到下一动作的间隔：`demoMode.store-CYJllHvf.js:37` 的录制函数 `zK` 内部 `h(x)` 先计算 `M=Math.floor(Qh()-v)`，再执行 `F=t.value.length-1; F>=0&&!t.value[F].delay&&(t.value[F].delay=M),t.value.push({...x,delay:0})`。即新事件到来时把间隔写到前一动作，当前事件先置0。`U.setMacro`逐项将`k.delay`传给`Pe.create`，保持该时序。滚轮、媒体、电源和位移事件保留为未知，不猜测转换。

`read_atk_macro` 返回指定槽的384字节及全部24字节按键区构成的CAS、已知动作和各按钮原映射。`apply_atk_macro`只写变化的10字节块，保留未使用宏尾字节；空白槽初始化已证实头。每块SET后立即GET该块核对，使每次最多遇到一个前次SET ACK；最终最多三次全384+24字节读回，不把ACK当成功。70动作完整事务mock模拟每个SET产生ACK。

`U.setKey` 证实class6为宏，value1为零基槽，value2经宏类型转换；固定次数直接编码。当前只开放repeat1..250，不开放253/254/255特殊循环/停止模式。`bind_atk_macro`的CAS同时包含目标槽全部内容和全部24字节按键，先确认宏可识别且非空，只覆盖目标按键4字节 `[6,slot,repeat,85-6-slot-repeat]`，保留其他映射，最终完整读回。目标未知映射或复杂快捷键先在普通映射界面确认，不直接覆盖。

`settings/mappings/shortcuts.rs`识别本应用可写宏三元组，返回`currentAction=macro`和可编辑状态；已有普通映射命令可将它改回普通功能，保留宏内容及快捷区。宏内容可反复读取和编辑，解除绑定不清空宏。

## 接口与验证范围

新增命令 `read_atk_dpi/apply_atk_dpi/read_atk_macro/apply_atk_macro/bind_atk_macro`；共用SettingsError。所有JSON变更对象拒绝未知字段。没有固件升级命令。

离线mock覆盖DPI边界、拒绝取整、相邻档位保留、CAS过期拒写、非目标字段变化导致读回失败；宏精确动作字节、尾区保留、70动作逐块ACK排空、相邻槽保留、完整CAS、未知头拒绝、失衡事件拒绝、绑定与解除绑定。主代理统一运行Rust测试，具体结果以本轮输出为准。设备CID/MID响应、EEPROM写入与行为效果需之后按实机验收顺序验证。

## 官网重新捕获与字节核验

主代理于2026-10-23使用Codex内置浏览器从官网真实Network响应保存资料，捕获时间为 `2026-10-23T11:52:37.349Z`。入口版本路径为 `https://bpcdn.atkgear.com/hub-v3/production/3.2.29/static/`。精确URL、捕获方式及时间见 `references/web/20261023-atk/capture.json`；离线计算的字节长度、SHA256、旧路径及旧SHA256见同目录 `manifest.json`。本次复核代理仅读取本地文件，没有联网或操作硬件。

| 文件 | 字节数 | SHA256 | 与references根目录旧快照比较 |
|---|---:|---|---|
| filter-BRWy5Cag.js | 145705 | 7f57dd4bbc2588f3ca7d5696e9537c2992f6665d686d4e7470c7923cf3005ec7 | 完全相同 |
| demoMode.store-CYJllHvf.js | 3696937 | 87f03d252ffc4dd24931a5765744c51df14195d13c949913b61d3551b0cdd1d6 | 完全相同 |
| base-hid-device-BZL9Z9sP.js | 2673 | c11d860630cd426e9df3b05ec7a078eef80efe7fb18b3d139ab7f6e1dc01990b | 完全相同 |
| filter-DhMj_pHY.js | 87213 | b39ea32af8d5210bedb96beaad7511c472bef702d1edb4534838c7ae5a63ed78 | 完全相同 |

`filter-DhMj_pHY.js` 是另一个BITMOUSE控制器分支，不是`filter-BRWy5Cag.js`的新版替代。其注册采用 `solutionProvider:K.BITMOUSE`、usagePage65285（FF05）、usage1，协议主体 `h.SIZE=63/baseOffset=7`；本项目的COMPX EEPROM控制器使用16字节主体及报告ID8。DhMj的F1 V3 ULTIMATE+/ULTIMATE条目属于新PID4868/4872/4932及独立CID/MID映射；相同数值CID/MID不可跨供应商解释。它没有注册本项目101A（4122）接收器。

该分支虽然列出 `SetMacroKey=6` 及其数据结构，但控制器 `async getMacro(){return this.result(!0)}`、`async setMacro(t,e){return this.result(!0)}` 是空实现，不能用作本项目完整宏写入的额外证据。它的DPI命令编号、包长和身份查询也不适用于COMPX EEPROM。此次重新捕获没有改变已实现F1 DPI/宏协议，也没有补齐固件包身份验证或恢复流程；不据此扩大设备白名单或开放刷写。

## 官方改键目录追加移植

官网 `demoMode.store-CYJllHvf.js:19` 的 `RI/hA/pA/ok/Rm/Mm/ap` 是本轮改键菜单依据。`RI`禁用使用 `Mouse/Disable=[1,0,0]`，保留兼容原class0禁用读值。`pA`横向滚轮使用class3、值1/2；`ok`板载1–4为class9、值161–164，循环为255。只写目标映射4字节。

`Rm`定义20种媒体预设：亮度加减、媒体播放器、停止、播放暂停、上下一曲、静音、音量加减、邮件、主页、搜索、刷新、收藏夹、停止加载、浏览器前后退、计算器及我的电脑。每项是MediaKey类型2的Down/Up两条动作，value1/value2分别是消费页usage低/高字节；例如主页 `[2,0,35,2]/[2,1,35,2]`，不能丢失高字节。它们通过已实现的独立快捷区写入、验证，再写selector `[5,0,0,80]`。完整144字节CAS、未知区只读保留和写后完整读回继续生效；已知媒体可重新编辑或解除。

主代理追加捕获 `keyboard.code-map-Yh-IioYC.js`，文件SHA256为 `f0bf640e0503a019fe1637059ad46e24538f3817b51fe60f59478790d38bfded`，59207字节/59009字符；来源、URL和本地保存时间记录在manifest的additionalFiles中。其导出 `r=G=[F,M,y,O]` 是 `Mm`的准确键目录：104项，普通HID4–99排除50、菜单101、左右八个修饰224–231；F区仅F1–F12。没有因HID数值相邻擅自开放F13–F24或电源。单修饰按键按官方 `ap`转换成type0、bit mask；普通键type1。组合最多两个修饰及一个主键，受6条动作/20字节槽限制；额外修饰支持左右八种。

返回actions增加category，shortcutSupport增加keyboardOptions（label/value/category）。前端以返回列表展示和校验，不自行推断连续数值范围。新增离线测试核对104项编码读回、右Ctrl精确帧、媒体高字节/尾字节保留、先快捷块后selector事务和媒体解除。

## 官方性能与参数追加移植

`filter-BRWy5Cag.js:1` 的 `Ht.get/create`证实地址169长度10，五对值/85-value字段分别为按键去抖、Motion Sync、休眠秒数除10、直线修正和波纹修正。`Wt.get/create`证实地址10长度2的LOD原值及伴随校验。`demoMode.store-CYJllHvf.js` 的 `NF/IRe/DRe`及`SilenceHeight`决定选项：去抖0/1/2/4/8/15/20ms；休眠30/60/120/180/300/1200/1500/1800秒；三个开关0/1；LOD为1mm(raw1)/2mm(raw2)，PAW3950额外0.7mm(raw3)。没有把LOD raw3无条件套给PAW3395/3950Ultra。

`read_atk_performance`和`apply_atk_performance`返回共享SettingsSnapshot。CAS含准确CID/MID、Ht完整10字节及LOD2字节。仅写指定字段的两字节，保留其他字段；当前目标字段未知编码/校验时不可写，其他未知字段原样保留。最多三次完整12字节读回一致后成功。mock覆盖精确地址/秒数编码、非目标未知字段保留、陈旧状态、LOD型号门控、未知目标拒绝，以及非目标字段改变导致失败。

## 与官方全功能的具体剩余差异

已移植改键主体不代表官方全部功能完成。以下仍无本地完整产品流程：炽焰键及参数编辑；DPI指示色/灯效；板载配置主动切换、重命名、完整配置导入导出；传感器角度及启用；竞技/传感器模式及独立传感器休眠；滚轮去抖；传感器中心点、动态灵敏度、bhop和快速触发等按固件能力显示的进阶设置；设备恢复出厂、配对及固件升级。这些功能需要各自准确型号/固件能力门控、读写/CAS/读回契约和实际UI，不提供伪入口。已实现部分的真实写入和行为验收仍按ACCEPTANCE记录。

## 2026-10-23 实机只读定位

用户反馈便携版鼠标设置可打开，但基础参数读取失败、原配置不显示，未尝试修改。首轮诊断时官网也显示接收器休眠，官网GET18无input；原生探针GET16、设置和DPI请求均触及600ms有界I/O超时。首版探针没有区分WriteFile与ReadFile阶段，不能从这轮结果判断发送已完成，更不能据此归罪接口、放宽帧校验或宣称已发现协议缺陷。

用户明确回复设备已唤醒后，主代理关闭官网HID标签，执行一次精确接口的GET16及五次EEPROM GET。脱敏原始帧及耗时保存在 `docs/evidence/atk-read-diagnostic-20261023.json`，时间 `2026-10-23T15:49:10.235343+00:00`。

| 请求 | 发送/接收 | 读取结果 | 单次总耗时 |
|---|---|---|---:|
| GET16 身份 | 17/17字节 | CID/MID=2,36，ATK F1 PRO / PAW3950 | 4.717ms |
| GET08 地址0，长度10 | 17/17字节 | `01540451015400550055`，1000Hz、零基档位1、共4档 | 4.055ms |
| GET08 地址12，长度8 | 17/17字节 | `070700470f0f0037`，400/800 DPI | 3.892ms |
| GET08 地址20，长度8 | 17/17字节 | `1f1f00173f3f00d7`，1600/3200 DPI | 4.029ms |
| GET08 地址28，长度8 | 17/17字节 | 未激活档位原值均3200 DPI，保留 | 3.908ms |
| GET08 地址36，长度8 | 17/17字节 | 未激活档位原值均3200 DPI，保留 | 4.175ms |

Windows描述符与官网WebHID一致：FF02/usage2的input/output均report8，原生含ID长度17；FF04仅feature6，没有input/output。唤醒后同接口与同请求全部成功，当前证据支持休眠无响应是本次失败条件，不需要改EEPROM地址、身份门控或响应长度。新增脱敏帧回归验证实际CID、设置与DPI全部能被现有解析链读取。

ATK读取超时提示应明确“设备可能休眠，请移动鼠标后重新读取”；不自动发送唤醒命令或反复重试。本轮没有SET、配对、固件操作或真实参数写入。成功只证明指定时刻的Win32只读GET和对应字段解码，便携版桌面UI展示仍需新包复查，不能把原生探针等同完整UI验收。
