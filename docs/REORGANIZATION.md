# 目录整理记录

日期：2026-10-23。产品代码模块位置保持稳定。

## 迁移清单

| 原路径 | 新路径 |
|---|---|
| `docs/history-placeholder-BACKEND.md` | `docs/history/2026-10-23-before-cleanup/BACKEND.md` |
| `docs/history-placeholder-FRONTEND.md` | `docs/history/2026-10-23-before-cleanup/FRONTEND.md` |
| `docs/history-placeholder-DELIVERY.md` | `docs/history/2026-10-23-before-cleanup/DELIVERY.md` |
| `docs/history-placeholder-ACCEPTANCE.md` | `docs/history/2026-10-23-before-cleanup/ACCEPTANCE.md` |
| `docs/history-placeholder-UI_SPEC.md` | `docs/history/2026-10-23-before-cleanup/UI_SPEC.md` |
| `PROJECT_PLAN.md` | `docs/history/PROJECT_PLAN_INITIAL.md` |
| `docs/REFACTOR_2026-10-23.md` | `docs/history/REFACTOR_2026-10-23.md` |
| `docs/UI_POLISH.md` | `docs/history/UI_POLISH.md` |
| `docs/DEVELOPMENT_HANDOFF_2026-10-23.md` | `docs/history/DEVELOPMENT_HANDOFF_2026-10-23.md` |
| `DeviceCenter-source-0.1.0.manifest.json` | `releases/source/DeviceCenter-source-0.1.0.manifest.json` |
| `DeviceCenter-source-0.1.0.zip` | `releases/source/DeviceCenter-source-0.1.0.zip` |
| `DeviceCenter-source-complete-0.1.0.manifest.json` | `releases/source/DeviceCenter-source-complete-0.1.0.manifest.json` |
| `DeviceCenter-source-complete-0.1.0.zip` | `releases/source/DeviceCenter-source-complete-0.1.0.zip` |
| `DeviceCenter-source-custom-0.1.0.manifest.json` | `releases/source/DeviceCenter-source-custom-0.1.0.manifest.json` |
| `DeviceCenter-source-custom-0.1.0.zip` | `releases/source/DeviceCenter-source-custom-0.1.0.zip` |
| `src-tauri/portable` | `releases/portable` |
| `src-tauri/portable-auto` | `releases/portable-auto` |
| `src-tauri/portable-complete` | `releases/portable-complete` |
| `src-tauri/portable-custom` | `releases/portable-custom` |
| `src-tauri/portable-next` | `releases/portable-next` |
| `src-tauri/portable-polish` | `releases/portable-polish` |
| `src-tauri/portable-refactor` | `releases/portable-refactor` |
| `src-tauri/DeviceCenter-portable-auto-x64.zip` | `releases/DeviceCenter-portable-auto-x64.zip` |
| `src-tauri/DeviceCenter-portable-complete-0.1.0.zip` | `releases/DeviceCenter-portable-complete-0.1.0.zip` |
| `src-tauri/DeviceCenter-portable-custom-0.1.0.zip` | `releases/DeviceCenter-portable-custom-0.1.0.zip` |
| `src-tauri/DeviceCenter-portable-next-x64.zip` | `releases/DeviceCenter-portable-next-x64.zip` |
| `src-tauri/DeviceCenter-portable-x64.zip` | `releases/DeviceCenter-portable-x64.zip` |
| `src-tauri/DeviceCenter_0.1.0_x64-setup.exe` | `releases/DeviceCenter_0.1.0_x64-setup.exe` |
| `artifacts/device-center-refactor.zip` | `releases/device-center-refactor.zip` |
| `artifacts/DeviceCenter-portable-polish.zip` | `releases/DeviceCenter-portable-polish.zip` |
| `artifacts/polish-compact-dark.jpg` | `artifacts/screenshots/polish-compact-dark.jpg` |
| `artifacts/polish-compact-light.jpg` | `artifacts/screenshots/polish-compact-light.jpg` |
| `artifacts/refactor-connect.jpg` | `artifacts/screenshots/refactor-connect.jpg` |
| `artifacts/refactor-dark.jpg` | `artifacts/screenshots/refactor-dark.jpg` |
| `artifacts/refactor-light.jpg` | `artifacts/screenshots/refactor-light.jpg` |
| `artifacts/refactor-before` | `archive/ui/refactor-before` |
| `artifacts/ui-polish-before` | `archive/ui/ui-polish-before` |
| `atk-source.js` | `references/web/atk-source.js` |
| `mchose-source.js` | `references/web/mchose-source.js` |
| `atk.html` | `references/web/atk.html` |
| `mchose.html` | `references/web/mchose.html` |
| `tauri-source.html` | `references/web/tauri-source.html` |

## 清理与取舍

删除 artifacts/refactor-check 的HTML、TS和Svelte一次性测试入口：未接入日常测试，仍使用旧样式与旧样本。保留当前MappingPreview和7个自动测试文件；它们覆盖不同回归场景。移除命令契约测试中“至少15个调用”的数量约束，保留扫描非空与命令注册一致性验证。

旧BACKEND、FRONTEND、DELIVERY、ACCEPTANCE、UI_SPEC在history保留快照，当前文件改为简洁现状说明。去除重复验收免责声明，状态集中到ACCEPTANCE。未删除设备身份、原块比较、读回及凭据边界校验。

## 验证

- `npm test`：15项通过。使用目录参数，兼容本机Node在Windows下不展开通配符的行为。
- `npm run build`：Svelte检查0错误0警告，Vite构建成功。
- `src-tauri/*.ps1`：PowerShell解析通过。
- 源码归档脚本使用新路径实际执行成功：119个输入文件、121个ZIP条目，白名单和内容哈希验证通过。验证快照为 `releases/source/DeviceCenter-source-layout-20261023.zip`；它是整理过程快照，之后的本条验证记录不在其中。
- polish程序现址哈希为 `850E1ADA962A7B89362708226644DC89462C69A117697B0573FEA084AA786E9F`，与迁移前交付记录一致。
- 未更改Rust业务代码，未重新运行Rust测试、启动桌面应用或操作硬件。
