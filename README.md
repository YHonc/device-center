# Device Center · 桌面设备中心

一个 Windows 本地外设控制面板，将 ATK F1 鼠标、迈从 KX75 键盘和小米屏幕挂灯 1S 集成在同一应用中。

## 功能

- **鼠标**：电量、性能参数、已有档位 X/Y DPI、按键映射和设备宏。
- **键盘**：电量、休眠与性能设置、分层改键、内置灯效和单色 RGB。
- **挂灯**：米家扫码导入、开关、亮度、色温及本地预设。
- **桌面**：托盘电量、快捷控制面板、后台刷新、明暗主题和玻璃风格界面。

项目仍在开发中。已有设备只读与部分挂灯流程的实测记录；键鼠写入、完整桌面行为和长期性能尚待验收。具体支持范围见[设置说明](docs/SETTINGS.md)和[验收记录](docs/ACCEPTANCE.md)。

## 本地运行

技术栈：**Tauri 2 + Rust + Svelte 5 + TypeScript**。

需要 Windows、Node.js 22.12+、Rust、Microsoft C++ Build Tools（含 Windows SDK）和 WebView2 Runtime。

```powershell
npm ci
npm run tauri -- dev
```

仅预览界面用 `npm run dev`；浏览器预览不连接真实设备。

```powershell
npm test                    # 前端行为测试
npm run build               # 前端检查与构建
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run tauri -- build      # 构建桌面应用
```

本机 GNU 工具链的替代构建方式见[后端说明](docs/BACKEND.md)。仓库不包含预编译程序、本地工具链和设备凭据。

## 项目结构

```text
src/          前端界面与状态管理
src-tauri/    Rust 设备协议、后台服务与桌面集成
tests/        前端回归测试
assets/       应用素材
docs/         架构、协议、规划与验证记录
scripts/      辅助工具
```

[开发约定](AGENTS.md) · [架构](docs/ARCHITECTURE.md) · [开发规划](docs/ROADMAP.md) · [文档索引](docs/README.md)

## 许可与来源

采用 [GPL-3.0-only](LICENSE)。miIO 实现参考并改编自 QiLight，其他来源及许可见[第三方说明](THIRD_PARTY_NOTICES.md)。本项目为非官方工具，与设备厂商无隶属关系。
