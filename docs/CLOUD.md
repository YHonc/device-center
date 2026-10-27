# 米家扫码获取设备配置

2026-10-23。实现来源为 `references/qilight/src/cloud.rs`（快照868e09450d86167dfcb578c81fd9e02ab0b6c927，GPL-3.0）中的扫码、Cookie、RC4签名与家庭分页链；同时核对了 `references/mi-light-bar-controller/src/main/MiLogin.ts` 的二维码和设备列表流程。后者存在日志输出登录响应、硬编码userId等问题，没有原样移植。项目本身GPL-3.0-only，发行仍须保留QiLight许可与来源说明。

## 实际支持的方式

本地独立实现米家公开客户端的二维码登录请求流程，不是官方OAuth集成，也不能绕开云端。只在用户点击“米家扫码”时发起；需用户在米家手机应用扫码、确认并自行完成任何风控。账号密码、短信验证码、验证码识别、绕过安全验证均未实现。云端未返回完整确认资料时明确失败并要求用户在米家完成验证后重新发起。保留手动IP/Token入口。

步骤是用户选择地区并生成二维码、串行查询扫码状态、读取当前账号可访问家庭与共享家庭的设备，再按以下规则自动识别用户已授权的挂灯并调用原有导入命令，将该设备IP与Token直接交现有DPAPI保存。没有自动登录或账号密码输入。

## 扫码后自动识别

用户在确认新版登录成功后，追加授权扫码后自动导入匹配挂灯。选择逻辑使用前端已获的非秘密设备摘要，不新增Token接口：

1. 只考虑`eligible=true`且model精确为`yeelink.light.lamp22`的设备。
2. 优先匹配现有`get_lamp_config`返回的已保存IP；DPAPI解密仍仅发生在后端，不向界面返回已有Token。该IP唯一匹配时自动导入，多匹配保持人工选择。
3. 未匹配已保存IP时，如用户当前手动配置输入了目标IP，只匹配该目标；无匹配或有歧义时显示原因并保留选择，不悄悄导入其他IP。
4. 已保存IP和当前输入目标都不存在时，只有一个符合条件的设备才自动导入；多个候选不选择第一个，零候选显示未找到及不可用原因。已保存IP失配且没有另一个输入目标时，也不自动导入其他IP，保持手动选择。

目标IP来自既有配置或当前输入，不将本次用户提供的家庭地址写死在源码。导入失败保留尚未过期的登录会话，可重试导入或重新读取列表；取消和迟到响应不能触发自动导入。自动识别成功仅完成凭据保存，不额外发送灯光控制命令。匹配规则与界面取消流程由前端负责实现及针对性验证，具体结果以本轮测试输出为准。

主窗口销毁时后端调用`cloud::cancel_all`撤销扫码/已认证会话，标记取消并移除active；不等待正在进行的HTTP请求所持session锁，后续完成检查必须看到取消。托盘小窗单独关闭不撤销主窗会话。未应用编辑的关闭提示由前端负责，用户授权关闭后再销毁。纯单元测试分别覆盖pending/已认证会话在session锁被占用时撤销，未操作真实账号。

## 前后端契约

时间均为epoch毫秒。命令如下：

- `begin_mi_login({region})` → `{sessionId,qrImageDataUrl,expiresAt}`。地区白名单cn/sg/de/us/tw/ru/in，默认由UI选择cn。图片必须为PNG且小于等于2MiB，只返回PNG data URL，不向UI交远程登录/轮询URL。
- `poll_mi_login({sessionId})` → `{status:'pending'|'authenticated',expiresAt}`。一次最长12秒或剩余二维码有效期，以较短者为准，不在后端无限循环。前端串行轮询，组件退出立即停止并取消。
- `list_mi_devices({sessionId})` → `{devices:[{id,name,model,ip,eligible,reason}],fetchedAt}`。不包含Token、serviceToken、security、用户标识或原始设备JSON。
- 设备列表另返回`warnings:string[]`，前端必须显示。列表失败保留尚未到期的已认证会话，可由用户点击“重新读取列表”，不必强制重新扫码；失败前清除后端旧设备候选，不能继续导入失效的上次选择。
- `import_mi_lamp({sessionId,deviceId})` → 原有`LampConfig {configured,ip}`。只允许本次列表中eligible项，原始Token仅在Rust内部传给现有`save_lamp_config`，复用同一DPAPI、原子文件替换和挂灯队列。成功后关闭云会话；UI再取消是幂等操作。保存成功不代表局域网挂灯控制成功，仍需现有读取动作验证连接。
- `cancel_mi_login({sessionId})` → void。取消指定会话并丢弃内存状态，不影响不匹配的新会话。重复取消无副作用。

错误为固定中文String。HTTP错误最多带数值状态码，禁止返回完整reqwest错误、响应原文、Cookie、URL或Token。

## 协议及边界

二维码入口为account.xiaomi.com的longPolling/loginUrl，接收qr图片与lp轮询地址；确认后取得ssecurity和userId，跟随location并从Cookie取得serviceToken。登录与重定向地址限定HTTPS、xiaomi.com/mi.com及子域、无userinfo、仅443端口，最多10次重定向。客户端连接超时8秒、普通请求20秒，响应上限4MiB。

云API采用QiLight的RC4-drop1024、随机nonce、SHA256 signed nonce与SHA1签名。只调用代码固定的gethome、home_device_list及下述兼容device_list路径，无用户自定义URL或任意API。遍历homelist/share_home_list，各家庭独立分页、按did去重；限制100家庭、每家庭100页、总5000设备，游标缺失或重复直接失败，不保存部分查询结果。

设备列表兼容修复：原实现只接受`result.device_info`数组，用户实际遇到“米家设备列表格式异常”；未读取用户原始响应，因此不能声称已经确认其实际字段。现在接受`device_info`或`list`数组，验证家庭列表、设备行及分页状态，`has_more`支持布尔与0/1。结构不兼容时只报告固定已知字段名、JSON类型和未知字段数量，API拒绝只报告整数code，不输出字段值、动态对象键、原文、did或账号。

首家庭分页路径失败时，仅进行一次已知兼容路径`/home/device_list`（参数来自MiLogin.ts：getVirtualModel/getHuamiDevices/get_split_device/support_smart_home）。兼容结果仍须具备合法数组；结构错误、网络失败或API拒绝均明确失败，不把失败转换成空列表。成功回退保留首错误的脱敏说明，并通过warnings标明共享家庭及分页覆盖尚未验证；即使合法空列表也带该警告。兼容返回has_more时另提示列表可能不完整，绝不宣称已经列出全部账号设备。正常主路径返回合法空数组时不触发回退。

当前仅开放准确型号`yeelink.light.lamp22`且有私网IPv4及32位非全零十六进制Token的导入。此型号依据QiLight的挂灯1S支持与spec特殊映射，不代表本应用已做1S实机兼容验收。其他设备仍显示名称、型号与不可导入原因，不会把同账号全部设备当挂灯。

二维码有效期服从云端timeout并限1至300秒；登录确认后仅保留15分钟内存会话。到期异步定时器只持不透明会话ID/AppHandle，不持Session或密钥，到期删除active并取消会话；若网络操作正持会话锁，每250ms重查直到能释放，操作自身仍有超时与取消检查。取消、开始新登录、成功导入、退出进程也释放会话。主要长期密钥字符串使用Zeroizing，设备Token不写日志或前端；HTTP库Cookie Jar和JSON解析产生的临时分配不提供可证明的全内存擦除保证。只持久保存按用户授权规则匹配或由用户手动选择的设备配置，使用已有Windows DPAPI，不保存米家账号会话。

## 验证状态

单元测试覆盖域与协议白名单、Xiaomi JSON前缀、无效JSON、会话/权限错误脱敏、设备型号/IP/Token门控、前端摘要不含Token、会话过期、取消、非法地区和未知会话。构造HTTP客户端不发请求，测试没有真实账号调用、扫码、云设备访问或凭据写入。

列表回归额外覆盖device_info/list别名、null/对象/字段缺失、分页类型、schema脱敏、回退最多一次、回退失败不假空成功、空回退保留警告、主路径有效空列表不回退。这些为mock解析证据；后续用户已确认登录、导入及自动读取。异常与共享分页仍按 ACCEPTANCE.md 验收。

另提供显式ignored测试`cloud::tests::anonymous_qr_begin_cancel_network_smoke`，仅在单独选择后获取匿名二维码并立即本地取消，不调用扫码状态、登录确认或设备API。输出仅PNG字节数/尺寸及经过白名单校验的图片与轮询域名，不输出二维码、session secret、URL路径或查询参数。此测试成功也仅代表匿名二维码路径可用，不能代表登录或Token获取成功。

后续实测重点：风控、过期、取消、共享家庭分页及写入控制。上游未公开稳定OAuth契约，云端端点可变化；若失败，需进一步核对公开网页客户端请求逻辑并修复，不可承诺复制代码即可免云端或永久可用。
