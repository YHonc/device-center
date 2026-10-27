# 项目架构与后续开发交接

整理日期：2026-10-23。依据两个对话全部用户需求与最终回复，并核对当前模块、入口和交付文档。本次只整理文档，没有启动应用、访问设备、执行设置写入或重新运行测试。

## 1. 项目目标与基线

Windows 本地设备中心：将 ATK F1 鼠标、迈从 KX75 键盘、小米屏幕挂灯 1S 集成到一个轻量常驻应用，统一电量、设置、挂灯和托盘操作。使用 Tauri 2 + Rust + Svelte 5 + TypeScript + Vite，不是驱动网站的窗口封装。

对话来源：

- `01a1665d-c327-7150-b629-2d71d5b19fcf`，标题“整合外设控制并规划轻量 UI”：产品范围、协议接入、生命周期、图标、托盘电量和按键设置。
- `01a1670f-3ed1-7182-8ac6-71597402706e`，标题“UI skill+重构”：项目级 UI 技能、界面重构、重复入口收敛及最终 polish。

后续以当前源码为编辑基线；对话最新交付是 custom 功能基础上的 polish。`portable`、`portable-next`、`portable-auto`、`portable-complete`、`portable-refactor`、`portable-custom` 是不同阶段产物，不得按名称推断当前运行版本或验收程度。最新 polish 记录见 `UI_POLISH.md`，程序为 `src-tauri/portable-polish/DeviceCenter.exe`，压缩包为 `artifacts/DeviceCenter-portable-polish.zip`；本次未重新核验二进制哈希。旧安装包和源码归档不能自动视作包含 polish。

## 2. 架构和代码阅读地图

```text
Svelte 主窗 / 快捷面板
        │ 类型化 Tauri command + 状态快照/事件
Rust 应用协调、后台监控、托盘、窗口生命周期
        ├─ 按设备独立队列 → HID → ATK F1 / KX75
        ├─ 挂灯通信 → miIO / 有型号门控的 MIoT → 局域网挂灯
        └─ 米家扫码 → 设备发现/导入 → DPAPI 加密配置
```

| 位置 | 责任与阅读重点 |
|---|---|
| `src/main.ts` | 按 query 挂载 App、Compact 或 MappingPreview；依次加载 style、motion、refactor、polish CSS，后层会覆盖前层 |
| `src/App.svelte` | 主界面、状态和功能入口；日常操作与诊断分离 |
| `src/Compact.svelte` | 电量 → 灯光 → 更多设备设置；当前原生尺寸为 360×480 |
| `src/Overlay.svelte` | 抽屉/模态、退出动效、焦点及关闭交互 |
| `src/DeviceSettings.svelte`、`MappingPanel.svelte`、`settings.ts` | 能力驱动的设置、原值/变更预览、应用和错误反馈 |
| `src/MiCloudConnect.svelte`、`cloudSelection.js` | 扫码流程、唯一匹配自动导入；多个候选或不匹配时手选 |
| `src/TrayPreferences.svelte` | 托盘仅图标、鼠标、键盘、两者四种模式 |
| `src/api.ts` | 桌面命令边界；浏览器预览明确拒绝真实后端调用 |
| `src/snapshot*.js`、`listenerScope.js`、`windowCloseRoute.js`、`lampCloseGuard.js` | 快照启动/顺序、监听释放、关闭路由、待发送灯光操作保护 |
| `src-tauri/src/lib.rs` | 命令注册、HID、每设备队列、窗口创建销毁、托盘和应用集成 |
| `src-tauri/src/monitor.rs` | 后台轮询、状态、退避和提醒；不能依赖前端存活 |
| `src-tauri/src/settings.rs`、`settings/mappings.rs`、`settings/kx_mapping.rs`、`settings/mappings/shortcuts.rs` | 基础设置、鼠标映射/快捷键、键盘按层映射及事务验证 |
| `src-tauri/src/protocol.rs`、`crypto.rs`、`config.rs`、`cloud.rs` | 灯通信、加密、本地凭据与米家流程 |
| `src-tauri/src/tray_display.rs` | 托盘缓存和变化时重绘，未知/过期电量及提示文字 |
| `tests/`、`docs/evidence/` | 前端逻辑回归和历史隔离生命周期证据 |

## 3. 功能范围与证据边界

| 功能 | 已有实现/历史证据 | 后续不能省略的验证 |
|---|---|---|
| ATK 电量和基础设置 | 历史只读取得 95%、1000 Hz、已有四档中的第二档 | 新版、当前设备状态及真实写入；DPI 档位不是任意 DPI 数值 |
| KX75 电量和休眠 | 对话及 BACKEND 记录型号探测、电量 100%、休眠 120 秒 | 设置写入、实际休眠行为；充电含义未验证，保持未知 |
| 鼠标功能映射 | 六按钮、已核验功能、有限组合快捷键 | 实机按键行为；不覆盖未知映射与其他快捷键区域 |
| 键盘映射 | Default/Fn/Fn2 层普通字母数字及两项官方组合预设 | 实机全块读写；任意自定义组合链未闭合，不开放 |
| 米家与挂灯 | 登录、导入和读取已有用户反馈；唯一匹配自动选择已有代码与测试 | 用户“现在能自动读取”不细分选择过程，也不证明亮度/开关写入 |
| 生命周期 | 销毁/重建 WebView、后台继续工作已有实现与隔离证据 | 真实硬件、休眠恢复、通知和自启长期行为 |
| 最新 UI | polish 历史记录 50 Rust、15 Node 测试通过，浏览器布局检查通过 | 新程序未在该轮启动；桌面菜单、定位、动画和窗口操作仍待验 |

宏录制、固件升级、完整灯效编辑、任意 DPI 数值、任意 KX75 组合键均不能默认为现有能力。历史一次读取值只证明当时结果。

## 4. 必须保留的开发约束

1. 接入优先参考已有 GitHub 项目；不匹配时追踪官方网页的实际识别、继承链、报文和解析，再移植到本地。K99 不可直接套用为 KX75；WebHID collection 不可直接当原生 HID 接口。
2. 设备识别必须精确。ATK 当前已验证接口为 373B:101A / FF02 / usage 2 / interface 1；KX75 为 3554:FA09 / FF02 / usage 2 / interface 1，且每次操作核验型号签名。通用接收器名称不证明型号。
3. 设置必须“读取现值 → 预览修改 → 用户应用 → 重读比较完整原块 → 最小修改 → 写后全块读回”。保留非目标字节；陈旧快照拒绝写入；ACK 不等于完成；写后失败可能部分生效，提示刷新，不自动回滚或重放。
4. 同一设备整次事务共用锁，防止轮询抢包；不同设备独立队列，防止一台超时拖住另一台。只提供类型化命令，不添加任意原始 HID/RPC 入口。
5. 未知、离线、历史读数过期分别表达；失败不把预览值冒充设备现值。浏览器预览不生成假电量。挂灯滑条合并待发值，保留最终值和关闭保护。
6. Token 只在后端按当前 Windows 账户 DPAPI 加密保存，不回传已保存 Token，不写 localStorage/日志。扫码二维码及会话不持久化。诊断材料只保留脱敏结构。
7. 原生窗口关闭释放 WebView，后台服务继续运行；重建窗口通过快照与事件恢复。不要退回仅隐藏窗口的早期方案。
8. 用户当时要求在设备休眠期间只开发和离线验证。后续整理、UI 或 mock 工作不应自行启动会访问设备的应用或 ignored 硬件测试；实机验收单独确定范围。

协议字段、校验假设与具体限制以 `SETTINGS.md`、源码和实机证据共同判断；文档里的 fail-closed 假设不能升级成已证实固件协议。

## 5. UI 开发要领

- 用户反复指出的问题是系统边框、玻璃层次不足、长表单、重复诊断入口和生硬动画。目标是紧凑、清晰、操作顺序明确。
- 主窗用自绘窗口栏；首页只保留状态与常用灯控；键鼠设置进抽屉，米家连接用步骤弹窗；诊断唯一入口为“应用偏好 → 设备诊断”。
- 玻璃主要用于导航、浮层、控件，正文保持对比度。当前为不透明原生窗口内部 CSS 玻璃，不宣称已经实现桌面背景折射。
- 保留明暗主题、减少透明度、系统减少动态效果、焦点返回和未应用修改保护。Windows 原生托盘菜单仍由系统绘制。
- polish 当前用较小进出位移、进入 250ms，退出更轻；动画要在实窗观察，不能靠截图确认顺滑。
- UI 工作按实际用途读取项目 `.agents/skills/refactoring-ui/SKILL.md` 和相关 transitions 技能；历史“全套 skill”不表示每次都要加载全部技能。
- 当前 CSS 多层覆盖是后续维护风险。新增规则前核对加载顺序；集中整理样式应作为明确任务并做视觉回归。

## 6. 构建、验证与交付

前端常用命令：`npm run check`、`npm run build`；逻辑回归 `node --test tests/*.test.mjs`。原生使用 `powershell -ExecutionPolicy Bypass -File src-tauri/build-windows.ps1 -Action test`，发布用 `-Action release`，安装包用 `-Action installer`。执行前检查脚本及当前环境，不在同一工作区并发运行原生构建脚本。

本项目曾遇中文路径 GNU 链接问题，构建脚本使用临时盘符及 ASCII TEMP target。保留项目局部工具链，不改全局 PATH；以脚本实际输出定位产物。便携包需要完整目录、WebView2Loader.dll 和许可文件，系统仍依赖 WebView2 Runtime。

发布前记录源码状态、构建日志、产物 SHA256 和对应验证版本。旧包先保留，避免误覆盖运行版；切换时从托盘退出旧版，防止并发访问设备。GPL 及第三方来源见 LICENSE、THIRD_PARTY_NOTICES、src-tauri/UPSTREAM；不要把官网抓取 JavaScript 打入发行包。

历史性能：某次使用会话约 10 分钟平均私有工作集 101.7 MiB，窗口状态未固定；隔离零窗短样本约 5.6–5.8 MiB。前者不代表纯托盘，后者不代表真实设备下长期达标。后续分别测主窗、快捷窗、零窗托盘和恢复场景，并包含整个进程树。

## 7. 文档冲突和下一步

`BACKEND.md` 早期段落仍写隐藏 WebView/全局 HID 锁；当前源码有 destroy 和按设备队列。`FRONTEND.md` 早期扫码描述仍写手动导入；后续已增加自动选择。`DELIVERY.md` 同时保留旧反馈与后续确认，且快捷窗写 360×440；最新源码和 UI_POLISH 为 360×480。使用时按具体版本和后续证据解释，不能只读首段。

建议后续顺序：

1. 确定当前运行程序及待验包版本，建立统一版本/哈希记录，随后更新上述陈旧段落。
2. 对 polish 实窗验收入口、拖拽/关闭重建、托盘定位、动效、缩放、明暗主题与辅助偏好。
3. 在明确实机测试范围后，逐设备验收原值读取、单项写入、全块读回、实际行为、断连与并发修改；保留恢复所需原值。
4. 做真实设备下分场景性能、通知、自启及休眠恢复验证，再决定扩展更多协议功能。

历史对话曾在特定开发轮次指定 GPT-6.1 Sol / medium 负责编码。这是当轮分工，不覆盖当前个人规则；后续是否委派、模型和编辑权限应按当次明确授权处理。
