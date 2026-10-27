# 目录与架构

文档核对：2026-10-27。包含10月24日功能扩展及10月25日前端交互收敛；目录整理过程见 REORGANIZATION.md。

## 调用关系

Svelte 主窗/快捷窗 → `api.ts` 类型化 Tauri 命令 → Rust 服务 → HID 键鼠 / miIO 挂灯。Rust 后台维护状态快照并推送事件，窗口可销毁重建。米家模块完成扫码、设备发现和本地配置导入。

| 模块 | 入口 |
|---|---|
| 主窗、快捷窗、浮层 | src/App.svelte、Compact.svelte、Overlay.svelte |
| 基础设置、映射、扫码、托盘偏好 | DeviceSettings.svelte、MappingPanel.svelte、MiCloudConnect.svelte、TrayPreferences.svelte |
| 事件与窗口协调 | snapshotBootstrap.js、snapshotOrder.js、listenerScope.js、windowCloseRoute.js、lampCloseGuard.js |
| 原生集成与队列 | src-tauri/src/lib.rs |
| 后台与托盘 | monitor.rs、tray_display.rs |
| 设置协议 | settings.rs、settings/mappings.rs、settings/kx_mapping.rs、settings/mappings/shortcuts.rs |
| 挂灯与凭据 | protocol.rs、crypto.rs、config.rs、cloud.rs |
| ATK性能、DPI与宏 | src/AtkParameters.svelte、AtkDpi.svelte、AtkMacro.svelte；src-tauri/src/settings/atk_extensions.rs |
| KX布局与灯效 | src/KeyboardLayout.svelte、KeyboardLighting.svelte；src-tauri/src/settings/kx_extensions.rs |
| 灯光预设与编辑 | src/LampPresets.svelte、lampPresets.js、lampPresetEdit.js、lampControls.js |
| 编辑与异步状态 | src/dpiInput.js、macroDraft.js、lightingDraft.js、requestGate.js、mappingSearch.js |
| 原生快捷窗定位 | src-tauri/src/compact_position.rs |

`src/main.ts` 选择主窗、compact、映射预览或灯光预设预览。样式加载顺序为 style → motion → refactor → polish；动效辅助逻辑位于 motion.ts。当前快捷窗为320×320，早期360×480尺寸只适用于历史polish版本。下一轮样式维护可按组件整理，先保持现有视觉回归基线。

## 存放结构

```text
项目根目录             README、AGENTS、许可、前端构建配置
src/                   前端产品代码
src-tauri/             原生源码、配置、图标、构建与验收脚本
tests/                 自动行为回归
assets/                产品素材及来源
scripts/               可复用辅助工具
docs/                  当前文档
  history/             历史规划与交付快照
  evidence/            精选验收证据
releases/              便携目录、安装包、ZIP
  source/              源码归档与 manifest
artifacts/             screenshots、performance、validation
archive/ui/            重构前备份
references/            协议参考仓库；web/ 存根目录迁入的官网快照
.tools/                工具链、缓存、临时脚本和日志
```

产品代码路径本轮保持稳定；整理集中在文档、研究材料和生成物。历史文件里的原始路径按历史时点理解，迁移对照见 `REORGANIZATION.md`。工具链、依赖、研究和发行目录由 Git 忽略。
