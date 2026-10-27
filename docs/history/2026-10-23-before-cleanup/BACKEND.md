# 原生后端

后端位于 src-tauri，GPL-3.0-only。crypto/protocol 复用 QiLight（见 src-tauri/UPSTREAM.md 和完整 LICENSE）；ATK 查询参考 atk-tool-plus MIT（LICENSE-ATK）。未复用参考项目 UI、安装脚本、云登录。

## 接口

Tauri commands 仅接受类型化输入，无任意 HID 报文或 RPC 执行入口。

| command | 参数 | 返回 |
|---|---|---|
| list_hid_devices | 无 | id、vendorId、productId、usagePage、usage、interfaceNumber、manufacturer、product、batterySupported |
| read_battery | deviceId | batteryPercent、charging 可空、sampledAt 毫秒、model；ATK 含 voltageMv，KX75 含 chargeStatusRaw/fullStatusRaw |
| get_lamp_config | 无 | configured、ip 可空 |
| save_lamp_config | ip、token | configured、ip；不返回 token |
| read_lamp | 无 | power、brightness、temperature、sampledAt |
| set_lamp | change: power?/brightness?/temperature? | 执行并读回确认后的灯状态 |
| open_official | brand: atk/mchose | 无 |
| get_autostart | 无 | boolean |
| set_autostart | enabled | boolean |

token 以 Windows 当前账户 DPAPI 加密后保存 app_config_dir/lamp.dpapi；JSON 序列化明文使用后清零，不记录日志。配置仅保存，不探测设备。IP 只允许私有 IPv4。握手和 RPC 到固定 UDP 54321，包含超时、校验和、设备ID、请求序号验证；网络失败至多重试一次。电量 ID 是 HID 路径编码，仅在本机内存中用于重新枚举后精确匹配。

## 协议范围

- ATK：373B:101A、usagePage FF02、usage2、interface1；按已纠正的官网继承链使用 Output report08 电量查询，输入回复限时500ms等待。此精确接口已本机读到95%、未充电、4109mV。FF04在Windows hidapi调用失败，禁止作为候选。回复长度、范围校验不等于完整帧认证；不能由接收器推断 F1 子型号。
- KX75：3554:FA09、usagePage FF02、usage2、interface1；官网WebHID的FF04过滤在Windows原生调用上须映射到已验证可用的FF02 collection。通用接收器产品名不直接证明型号，每次操作先发送只读 getPassWord 型号探测，并校验精确公开型号签名 03 00 00 00 01 C7（本机已成功）。只在型号匹配后发送 report13/getBattery 或类型化设置；仅接受单包序号0、长度20并校验字节和。充电码语义尚未实机验证，charging 返回 null。
- 灯：先尝试 legacy get_prop；只有方法不支持或已认证状态形状不兼容才尝试 MIoT。MIoT 必须通过 miIO.info 确认 yeelink.light.lamp22，使用 QiLight 的 siid2/piid1,2,3 主灯映射。未知型号不套用映射。亮度1–100、色温2700–6500。设置 ACK 后有限次读回目标字段，未达到目标返回错误；部分字段可能已生效，不做未经请求的回滚。

托盘支持显示、退出；关闭窗口隐藏 WebView。当前没有销毁 WebView，未证明常驻内存预算。后台监控由 monitor.rs 提供低频查询、事件和去重低电量提醒。开机启动默认不注册，只有显式调用 set_autostart 才修改。

## 构建与验证边界

项目工具链目录 .tools，不修改全局 PATH。执行 `powershell -ExecutionPolicy Bypass -File src-tauri/build-windows.ps1 -Action test`；release 先构建前端再生成原生 exe。dev 需另开终端 `npm run dev`。当前 bundle.active=true，targets=nsis。使用同脚本 -Action installer 生成当前用户安装包，包含 GNU WebView2Loader.dll 和许可；安装器尚未签名。已通过隔离解包验证，未在本机执行安装/卸载，不能称安装卸载验收通过。

全新开发环境可在项目根设置进程级 `CARGO_HOME=$PWD/.tools/cargo`、`RUSTUP_HOME=$PWD/.tools/rustup`，从官方 win.rustup.rs/x86_64 下载 rustup-init.exe，运行 `-y --no-modify-path --profile minimal --default-toolchain stable --default-host x86_64-pc-windows-gnu`；完整 WinLibs GCC POSIX x86_64 UCRT zip 从 brechtsanders/winlibs_mingw GitHub releases 下载，解压至 .tools/winlibs。不要将工具目录加入系统/用户全局 PATH。本次实际工具版本应以构建日志与 Cargo.lock 为准。

GNU 的 dlltool/linker 无法处理本项目中文路径，因此构建脚本临时 subst 到空闲盘符、统一工具和 sysroot 路径。Cargo 会将依赖产物路径还原到物理目录，故最终产物使用真实 ASCII 路径 `%TEMP%/device-center-target-<工作区哈希>`；脚本输出准确路径，并在结束后清理盘符映射。release 可执行文件位于该目录 release/device-center.exe。构建期间不要并发执行同一脚本。

MSVC 首次 test 在依赖 build script 阶段因 link.exe 不存在失败；本机标准 VS Build Tools 与 Windows SDK 目录亦不存在。LLVM-only MinGW 与 Rust GNU 默认运行库不匹配（缺 gcc/gcc_eh），不能据此宣称应用编译失败。完整 GCC 工具链构建结果由本次验证记录补充。纯解析测试和编译成功均不等于设备兼容、系统通知或自启已实机验收；未提供挂灯凭据，不执行挂灯操作。

2026-10-23 最终验证：完整 WinLibs GCC + Rust GNU 全测试 28 passed / 0 failed / 1 ignored；独立 ignored 只读实机探针读到 ATK 95%、4109mV、1000Hz、DPI档位1/4档，KX75 精确型号签名、电量100%、休眠120秒。KX充电码保留原始值且 charging=null；未进行任何实机设置写入。测试日志在 .tools/cargo-final-test.log / hardware-read.log。release 优化构建成功，前端 svelte-check 0错误0警告；便携目录 src-tauri/portable 含 exe、WebView2Loader.dll 和许可。便携 exe 已启动并存活5秒后关闭，此为启动冒烟，未代替窗口交互、通知、自启、米家登录或挂灯实机验收。Tauri 默认只给普通bin链接CommonControls6资源，本项目build.rs补齐了unit-test所需资源，解决测试可执行文件加载入口问题。

同日后续修复：用户真实扫码后遇到设备列表结构异常，cloud.rs 已补充合法 schema 兼容和单次旧接口回退，新增 mock 回归；修复后真实账户列表仍待用户再次核验。窗口改为无系统装饰并授权自绘标题栏的拖拽/最小化/最大化/关闭，保持不透明原生窗口及内部 CSS 玻璃以降低 Windows 材质兼容风险。电量超时仅一次150ms后重试，身份/校验失败不重试；所有 HID 手动/后台/设置操作共享同一互斥锁。上述版本 Rust 实际测试为30 passed /0 failed /2 ignored（硬件与匿名网络测试默认忽略）；没有再次执行账户登录或设备设置写入。

修订版交付：src-tauri/portable-next（保留旧portable）；压缩包 DeviceCenter-portable-next-x64.zip；安装器 DeviceCenter_0.1.0_x64-setup.exe。最终前端检查/构建、Rust release、NSIS成功；安装器通过 NanaZip 工具隔离解包验证，包含 exe、WebView2Loader.dll、GPL/ATK许可、UPSTREAM和THIRD_PARTY_NOTICES及卸载程序，未执行安装/卸载。新版便携exe在无subst盘、无.tools PATH环境启动存活5秒，进程树未连接本地开发端口1420/5173/4317；这是启动检查，不是完整交互验收。安装包内exe与便携exe仅有Tauri的3字节bundle类型标记差异（NSS/UNK），loader SHA256一致。

| 产物 | SHA256 |
|---|---|
| portable-next/DeviceCenter.exe | 0FDFD103CFB971E830D5BBFA4ACBE328DC24978A2C1E79C30AB0E6FD1EC91267 |
| DeviceCenter_0.1.0_x64-setup.exe | FDA6A86DBFA50DEAA8FCDB705F0D1DE432F7BEF900B5DC1872B1D9A426C209BA |
| DeviceCenter-portable-next-x64.zip | BECBC61AFD46DCB5970D29D022D33AF35ED8DDB9B6D26904EF9607ABE071788E |

最终构建/解包/启动日志分别位于 .tools/nsis-final-build.log、nsis-final-bundle.log、nsis-final-payload-list.log、nsis-final-extract.log、portable-next-smoke.log。未签名、未完整实测安装卸载、米家登录后设备列表、灯控制、设置写入或长期性能。

自动导入修订版另存 src-tauri/portable-auto，保留用户运行的portable-next。用户已确认前版米家登录可用；此版新增前端对唯一兼容灯/已保存目标的自动导入，相关前端4测试由主代理验收。Rust云相关测试8通过、1网络测试忽略（cloud-auto-unit.log），无网络/账户操作。installer构建成功（nsis-auto-build.log）。

本次获得明确验收授权后，以全新 .tools/install-smoke-ec3283d28ea44583936981f92a9e07ee 目录执行安装 `/S /NS /UPDATE /D=...`、卸载 `/S /UPDATE _?=...`，不启动安装应用。前置条件为产品/卸载/AppUserModelId/自启相关键不存在及已有WebView2；保护期间仅检查文件元数据、进程ID/启动时间，不读取令牌或云会话。安装/卸载均退出0，目录已清理，产品/卸载键恢复不存在，保护配置/快捷方式元数据不变，现有便携PID52984及启动时间保持不变。证据为 .tools/install-validation-auto.log；验收脚本 src-tauri/validate-installer.ps1 遇到相关键已存在或并发变化即停止。此项覆盖当前用户模式的受保护静默文件/注册表流程，不能替代全部交互安装、升级或用户主动删除数据测试。

| 自动导入产物 | SHA256 |
|---|---|
| portable-auto/DeviceCenter.exe | F393DDA3B924B3968C98FF0E59B79113179128CC60FB924778EE55063434E58B |
| DeviceCenter_0.1.0_x64-setup.exe（本次更新） | 2D48D0AB8723F965DEF79B7B4D2D9934FA7B2C8600E0DBE398716A99B7E2E3AA |
| DeviceCenter-portable-auto-x64.zip | 992948875059E6514194A27DFF0F86AFE927236354F2CA2FBE40862A5BCC2C7B |
