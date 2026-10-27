# 本地键鼠设置

## ATK 单键功能映射

设置先读取原值，用户预览后应用，后端检查完整原块并最小写入、完整读回。当前实现和实机验收状态见 ACCEPTANCE.md。

实现入口`src-tauri/src/settings/mappings.rs`。`read_button_mappings({deviceId})`返回`{deviceId,model,sampledAt,stateToken,buttons,actions,unsupported}`；每按钮含`id,label,currentAction,currentLabel,editable,rawHex`。`apply_button_mapping({deviceId,change:{buttonId,action,expectedState}})`只允许一个物理按钮和一个已核实功能，返回完整新快照。前端只显示原值与预览，用户点击应用才调用；未知原映射`editable=false`，后端同样禁止覆盖，不重建默认键图。

官方依据均为`references/filter-BRWy5Cag.js:1`：U的`buttonKeys`/`buttonKeysAddressMap`及`getCurrentKeyMatrix`、`setKey`，`zt.get/createOne`，枚举`ButtonKeyClass`/`ButtonKeyValue1Mouse`/`ButtonKeyValue1Dpi`/`ButtonKeyValue1Wheel`，基类C的`createDataView`。6个物理键为left/right/center/side1/side2/bottom；地址96、100、104、108、112、116。原映射每键4字节`[class,value1,value2,(85-class-value1-value2)&255]`。读取三个8字节块（地址96/104/112）；写入仅目标键4字节，保留另外20字节以及所有独立快捷键/宏区。

当前目录含19项：禁用[1,0,0]（读取兼容旧全零），左/右/中/侧1/侧2鼠标按钮[1,1/2/4/8/16,0]，DPI循环/加/减[2,1/2/3,0]，滚轮上/下[11,1/2,0]，回报率切换[7,0,0]，横向滚动[3,1/2,0]和板载配置1–4/循环[9,161/162/163/164/255,0]。actions带category用于分类。界面使用中文功能标签，不要求输入原始字节；未知原映射及字段配对只读保留。

报文沿用17字节Output包，GET08、SET07、地址大端、长度8或4、最多10字节数据及同一末字节算法。接收严格校验报告ID、命令、状态、地址与长度；跳过至多4个其他命令/写ACK。每次应用重新读取完整24字节并比较expectedState，只构造目标4字节；写后最多3轮完整24字节读取一致才成功，任何非目标字节变化也失败。设备门控沿用已核验的ATK373B/101A、FF02、usage2、interface1，每设备同锁覆盖读取、CAS、写入与读回。没有自动恢复默认、批量覆盖或自动重试写入。

组合快捷键追加实现：官方`setShortcutAction/_getShortcutAction`使用各按钮独立地址256+32*i、两个10字节块、最多6个3字节动作及计数/字段校验。`demoMode.store-CYJllHvf.js:19`确立HID修饰键224..231映射1/2/4/8/16/32/64/128及普通键/修饰键Down/Up；filter中的st.create为Down设置0x80、Up设置0x40，普通键低位1。当前开放A-Z/数字4..39及最多两个不同左侧Ctrl/Shift/Alt/Win，形成2/4/6动作（修饰按下→主键按下→主键抬起→修饰逆序抬起）；不宣称任意动作序列。原有复杂/未知序列只读保留。

ATK读取扩展为24字节映射+6*20字节快捷键区，完整144字节CAS。`change.action='shortcut'`时另传`shortcut:{keyCode,modifiers}`，返回`shortcutSupport`及每键当前已知`shortcut`。写入保留快捷区未使用尾字节，先写该按钮两个10字节快捷块并整块读回，再单写目标映射[5,0,0,80]启用；最后全144字节读回一致才成功。普通功能修改不覆盖任何快捷区。没有默认重建或失败后自动清空。

### KX75 原块按层映射

官网`purify.es-CxLZ1Htw.js:54`中`df`将KX75配置分派到`$5t`，`:4334`导出`keyList:o6`、`settingCommonParser:H5t`。o6准确96槽*4字节=384；`:4327`的KX75 `commonObj:$vt`含126字节space15和fix[90,165]，完整写块512字节。`kx_mapping.rs`明确列出原始96槽顺序，保留所有space/noChange/Fn槽以及另外128字节，不使用官网默认键表填补原值。

`read_keyboard_mappings({deviceId,layer})`、`apply_keyboard_mapping({deviceId,change:{layer,keyId,action,expectedState}})`支持Default/Fn/Fn2，按键返回与ATK同类摘要及rawHex，未知4字节映射只读。GET41请求YY=0/16/32；完整读取37个14字节载荷并取原512字节，确认5AA5尾标记。SET01分37包，每包等待匹配command/seq的ACK，写后最多两次原512字节全读回。尾包只有8字节数据加6零填充，但YY是层base14/30/46，不能照休眠协议写成8：官方splitCommand把未补零chunk复制给getWirelessYY，reverse仅作用副本，尾A5非零导致trailingZero=0。原始数据不反转。型号使用准确FF02/usage2/interface1并重新probe6字节KX75身份，全事务同device_queue锁。

KX75当前移植官网四类动作：基础104项、鼠标7项、媒体7项、扩展48项，另含全零禁用；保留已有A-Z/数字及任务管理器、Windows安全屏幕ID。actions带base/mouse/media/extended分类。原厂Fn特殊值可识别为可替换原值，但不生成新的原厂功能；Fn/Fn2层不允许分配Fn/Fn2。任意组合键和设备宏仍缺完整编码/保留契约：4320的鼠标buttonValue不能拼接为键盘四字节映射。详细依据见`evidence/kx-extension-protocol.md`。

mock回归包括精确GET/SET字节、未知非目标键逐字节保留、原未知目标拒绝、CAS过期不写、完整映射不一致失败、错误响应及未支持功能拒绝。普通测试不运行ignored硬件或云探针，在线只读调试另按当前授权执行；实际结果集中在ACCEPTANCE.md。

功能写入实测仍在最后进行；授权的只读调试不代表设备写入验收。

## 类型与调用

- `get_device_settings({deviceId})` 重新枚举指定设备并查询现值，ATK读取10字节设置块，KX75读取128字节性能块。
- `apply_device_settings({deviceId,change})` ATK接受 `reportRateHz`、`dpiLevel`；KX75接受`sleepSeconds`、`reportRateHz`、`lowLatency`、`macMode`、`winLock`。后三项change值为布尔，快照fields以关闭0/开启1的select选项表示。`lightingBrightness`拒绝写入，灯效使用独立命令；未知字段也拒绝。
- `change.expectedReportRateHz`、`change.expectedDpiLevel` 必须来自最后显示的现值。应用前重新读取，任一变化或缺失返回 `stale_state`，要求刷新、重新预览。
- 返回 `{deviceId,model,fields,sampledAt,verifiedOnHardware:false,stateToken}`。前端保存stateToken，并在应用时原样作为`change.expectedState`传回，后端比较完整设置块，避免覆盖其他客户端对非目标字段的更新。ATK兼容上述两个预览现值字段；KX75必须提供完整token。fields 提供 key/label/kind/value/available/reason/min/max/step/options；value 为未知时是 null。options 为 `{label,value}[]`。
- 错误是 `{code,message}`，区分参数无效、接口未支持、断连、连接错误、超时、设备非零状态、无效回复、预览过期、读回不一致。写后失败不得以预览值覆盖最后确认值；需提示设备可能已写入并刷新。

## 扩展命令

ATK扩展入口为`src-tauri/src/settings/atk_extensions.rs`。每次事务重新识别CID/MID，只接受已列出的F1型号，再使用与电量/基础设置相同的设备队列。

- `read_atk_performance({deviceId})`返回共享SettingsSnapshot；`apply_atk_performance({deviceId,change:{key,value,expectedState}})`单次修改一项。支持去抖0/1/2/4/8/15/20ms、Motion Sync、直线修正、波纹修正、休眠30/60/120/180/300/1200/1500/1800秒和抬起高度1/2mm（非Ultra PAW3950另有0.7mm）。读取Ht地址169的10字节和Wt地址10的2字节，CID及完整12字节参与CAS；仅写目标字段及85-value配对的2字节，再完整读回。未知目标字段只读，其他未知字段保留。

- `read_atk_dpi({deviceId})`读取基础10字节和地址12/20/28/36的四组8字节档位。`apply_atk_dpi({deviceId,change:{level,xDpi,yDpi,expectedState}})`修改一个已有档位的X/Y数值，保留同组另一档位及所有其他字节，完整读回后返回。档位数不在此接口扩展。
- DPI范围由CID/MID确定：Dragonfly F1 PRO/PRO MAX为100–30000，F1 PRO/PRO MAX为50–36000，F1 Ultimate/Ultra/EXTREME/LEVIATAN为10–42000。普通型号在30000以内步长50，以上步长100；Ultra编码型号在10000以内步长10、30000以内步长50、以上步长100。前端以快照`min/max/stepSegments`为准，不能把“任意DPI”解释为任意整数。
- `read_atk_macro({deviceId,slot})`读取0–11号宏槽及六键映射。宏槽地址为`768+384*slot`，每槽384字节。返回`editable`、`actions`、`maxActions`、`buttons`和完整`stateToken`。
- `apply_atk_macro({deviceId,change:{slot,actions,expectedState}})`支持最多70个动作，每个动作包含`kind`（key/modifier/mouse）、`state`（down/up）、`code`和`delayMs`。普通HID键码4–115，修饰键224–231，鼠标按钮1/2/4/8/16；按下/抬起必须配对，`delayMs`表示动作执行后的延迟，为16位毫秒且非末动作至少1毫秒。只发送发生变化的分块，每个SET后立即GET确认，最后完整读回宏槽与24字节映射（最多3轮）；保留槽位未使用尾部及所有按钮映射，未知原宏保持只读。
- `bind_atk_macro({deviceId,change:{slot,buttonId,repeat,expectedState}})`把已读回的非空宏绑定到一个物理按钮，重复次数1–250，仅写目标4字节映射。宏内容和24字节映射共同参与冲突检查与完整读回；没有无限循环模式。

宏面板支持用户显式开始的面板内录制、动作列表编辑和JSON导入导出预览。这些操作只改变本地草稿；用户点击写入并取得完整读回后才算保存到设备，再单独绑定按钮。录制不采集面板外的后台按键。

KX75灯效入口为`src-tauri/src/settings/kx_extensions.rs`。`read_keyboard_lighting({deviceId})`返回性能/颜色token、当前模式、参数、rgb和逐模式可编辑标记；`apply_keyboard_lighting({deviceId,change:{expectedState,mode?,brightness?,speed?,multiColor?,rgb?}})`复用型号探测和分包ACK/读回路径。

模式为0/1/2/3/4/5/7/8/10/11/12/13/15/16/17。`mode`修改[9]类型为内置和[10]模式；参数从58字节后的各模式两字节槽读取，模式15位于84、16位于88、17位于90。亮度/速度范围1–4，按模式能力开放；速度只改高半字节，单色/多色切换只改低半字节（0或7）。无目标变更不发送SET。

RGB依据官网`df`为KX75选择的`uae/hae`默认解析器及真机帧实现。GET49为35包、490字节配置，每模式颜色位于`mode*21`的RGB三字节。颜色成功读取时token覆盖性能128+颜色490字节，共618字节、1236个hex字符；先全量CAS，仅改目标RGB。SET09在原490字节配置后构造14字节零和`[0,0,90,165,十个零]`，共518字节37包，尾包YY4。该写运输尾部不参与CAS或GET读回，读回比较490字节配置区，再修改性能块并核对两块。只有允许调色的内置模式且单色时接受RGB。颜色读取失败仍保留性能快照并返回明确`colorError`（token仅256个hex字符），没有完整颜色token就不能写RGB。逐键DIY、音乐/GIF尚未开放。

固件页说明当前缺失的升级契约；应用保留现有`open_official({brand})`官方驱动入口。项目当前没有原生固件升级命令；不读取或执行未知固件二进制。升级分包、校验、版本与恢复流程尚待完整证据。

## ATK 接口及报文依据

官方静态快照 `references/filter-BRWy5Cag.js:1` 的 U/controller 注册 `VID 373B / PID 101A / usagePage FF04 / usage 2`。本机原生HID实际配置接口由只读EEPROM响应确认是 `373B/101A/FF02/usage2/interface1`，生产设置白名单使用此精确组合；不按品牌匹配。前端 deviceId 是本次枚举 path 的十六进制；后端重新枚举，禁止打开任意前端路径。

本机Output GET EEPROM实际成功，解出1000Hz、零基DPI档位1、已有4档，原块`01540451015400550055`。因此将此精确接口接入设置能力；仍未进行实机SET及写后读回，不把只读成功表述为完整设置验收。仅测试构建中的`diagnostic_read_sync`保留用于精确接口GET诊断，不包含SET。

同文件 `ge` 结构定义 EEPROM 地址 0000、长度10，字段为 `[reportRate,85-reportRate,maxDpi,85-maxDpi,currentDpi,85-currentDpi,bhop,bhopCrc,buttonOperationMode,buttonOperationModeCrc]`。回报率映射只开放本机1K接收器的125/250/500/1000 Hz。`references/demoMode.store-CYJllHvf.js:40` 中档位选择限制在0至dpiList.length-1并直接传给setMouseDpiConfig，所以 `dpiLevel` 是已有档位的零基索引，不是 DPI 数值；已有档位数安全限制1至8。

原生请求17字节：报告ID08、命令08（读取）或07（写入）、状态0、地址0000、大端、数据长度0A、10字节数据、校验。末字节为 `(0x4d - sum(payload[0..15])) & 255`，其中 payload 不含报告ID。读取数据全零。调用链已纠正为U→Or→dr（`base-hid-device`导出的E），promiseRun默认feature=false，因此使用Output `write`发送、input report读取；Feature默认true属于独立升级控制器Ka，不能挪用于普通设置。

读取仅接受17字节、报告ID08、命令08、地址0000、长度10、非零状态拒绝，并校验上述三个字段配对与范围。**85-value 是官方字段配对约束，不是已经确认的响应包CRC算法。** 官网没有完整响应包校验及状态码说明，当前对非零状态采取保守拒绝；真实固件回复长度、状态含义和接口仍需实机只读验收。

写入先读取完整10字节，核对预览现值，只改指定字段及伴随值，保留其他字节。发送后最多重新读取3次，只有完整10字节一致才返回成功；不把写ACK当成成功。每次读取最多跳过4个无关或前次写入ACK，每个input读取200ms超时，任何读取/连接错误立即失败，无无限重试。无变化则返回已确认读取结果，不发送写入。应用与该设备的电量查询共享按deviceId（枚举路径）索引的HID互斥锁。不同设备使用独立队列；同设备的型号识别及整块读取、修改、写入、读回验证由同一锁覆盖，避免其他轮询抢包。

## KX75 与验证边界

KX75门控为3554/FA09、FF02、usage2、interface1；当前不使用产品字符串白名单。每次读取/应用在打开设备后必须调用`verify_kx75`，只读GET型号标识必须精确匹配`03 00 00 00 01 c7`，才允许后续块操作。身份查询单次有界等待2200ms，不重发；最新实机超时记录见ACCEPTANCE.md。依据官网`:3881`的getPassWord和Bat.KX75注册，不能仅据产品名称推定型号。

`references/purify.es-CxLZ1Htw.js:54` 的Rpe/getPerformance/splitCommand/waitForAck以及`:4334`的P5t/K5t确立128字节性能块协议。GET原生请求 `13 44 01 [16个00] 58`，通过普通HID output write发送。实现要求10个20字节回复，序号0至9，分别拼接[5..19)的14字节，取前128字节。10包是由128/14推导的保守门槛，等待真机捕获核实；读取回复YY官网未使用，实现也不依赖YY。

休眠位于原块[24]。`:4644`界面显示raw/2、写入minutes*2，0至20步长0.5；`references/theme-DuG9L1_k.js:97,452`确认中英文标签分别为分钟/min，0为永不休眠。因此API使用整数`sleepSeconds`：0至1200，步长30，raw=seconds/30。

KX75基础性能按官网`performanceSetting`及P5t/K5t字段实现：`reportRateHz`映射[1]的1/2/3为250/500/1000Hz；TopSpeed（`lowLatency`）映射[3]的0为开、2为关；`macMode`映射[27]的2为开、0为关；`winLock`映射[15]的1为开、0为关。开启Mac模式同步把Win锁清0，Mac模式下请求Win锁开启会拒绝；快照禁用其控件并给出原因。选中修改的字段原编码未知时保持只读，其他未知字节原样保留。

基础性能接口先重读并以expectedState比较整个原块，仅修改明确选择的字段以及开启Mac时关联的Win锁。写入10个20字节包：`13 04 0a sequence YY [14载荷] checksum`，前9包YY14，末包YY2且补12个零，checksum为前19字节8位加和。逐包等待command04和对应sequence的ACK；中断立即失败，无自动重放。随后最多2次重读完整128字节，一致才成功。读取与写ACK目前还保守要求20字节与加和校验；**官网没有验证响应/ACK的校验算法，此约束属于fail-closed实现假设，必须以真实回复核验，不能称为已证实响应协议。** 基础接口的`lightingBrightness`字段保持不可写，灯效扩展使用独立命令和快照。

`settings.rs` mock测试覆盖GET精确报文、写报文校验、保留字段、跳过ACK与读回成功、预览过期不写、三次读回不一致、长度错误、超时、回报率范围、DPI档位范围、未知JSON字段，以及KX75完整分包写入与全块读回、坏序号、校验错误、中途ACK超时与完整块CAS。测试不连接HID，不证明真实设备兼容。待验收：官网同值对照、只读回复捕获、点击应用与读回、断连/休眠、官网并发修改、不同固件、写后失败恢复。
