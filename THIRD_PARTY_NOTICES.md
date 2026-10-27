# 第三方来源与许可说明

记录日期：2026-10-23。以下固定提交用于追溯研究和改编依据；`references/` 是本地研究快照，不随应用打包。项目原生包 `src-tauri/Cargo.toml` 声明 `GPL-3.0-only`。本文不替代完整许可证，也不代表依赖许可审计或发布验收已经完成。

## QiLight：miIO 通信来源

- 仓库：https://github.com/wangzitao21/qilight
- 固定提交：`868e09450d86167dfcb578c81fd9e02ab0b6c927`
- 许可：上游 `LICENSE` 为 GNU GPL version 3。
- 相关上游模块：`src/protocol.rs`、`src/control.rs`、`src/spec.rs`、`src/crypto.rs`、`src/config.rs`。
- 本项目的 Rust miIO 协议、加解密及控制实现复用或改编上述工作。核对时 `src-tauri/src/crypto.rs`、`src-tauri/src/protocol.rs` 与固定上游对应文件逐字节一致；配置等其他模块包含适配改动。本项目另增加面向 Tauri 的设备状态和交互边界。适配日期为 2026-10-23；具体差异以本项目源码和版本记录为准。

分发时保留上游归属和修改说明，附完整 GPLv3 正文，并按 GPLv3 提供对应源码及构建材料。根目录 `LICENSE` 与 `src-tauri/LICENSE` 均保留上游完整 GPLv3 文本。GPL 正文必须包含在发行源码和适用的发行物中，不能仅以本文件或上游链接代替。上游项目作者及贡献者的权利不因改编而转移。

## ATK Tool Plus：HID 研究依据

- 仓库：https://github.com/papelblume/atk-tool-plus
- 固定提交：`a1a9e4663705bcc0d71ded33c7783b9b0371c1f2`
- 许可：MIT。
- 用途：设备注册、HID 电量协议及报文处理研究；准确型号兼容性仍需验证。

上游许可原文如下；本项目如包含其代码或实质性部分，应随该部分保留此通知：

```text
The MIT License (MIT)

Copyright © 2026 Levon Kirakosyan

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

## 仅研究的其他项目

| 项目 | 固定提交 | 当前使用边界 |
|---|---|---|
| https://github.com/Anqing-None/mi-light-bar-controller | `d7ebf8b55153cd1c7d32d31eeef19e4fb10c1591` | 研究界面、登录和 miIO 调用；当前快照未找到独立 LICENSE，复制其代码前另行确认许可 |
| https://github.com/arutkowski00/mchose-hid-bridge | `ad74f3e75fd5ae099b2ea4af039678c54dea73ac` | 研究 K99/K99 V3 报文与描述符；当前快照未找到独立 LICENSE，不直接移植桥接代码，不把它当作 KX75 支持依据 |

ATK 和迈从官网入口、下载脚本与 HTML 仅用于协议定位；未授予本项目再发行权，不纳入发行源码或安装包。

本地键鼠设置由本项目根据官网协议行为独立实现，未分发官网下载JavaScript。ATK普通HID Output路径的精确身份、10字节设置块及已有DPI档位语义，以及KX75的128字节性能块/休眠单位来源记录于 `docs/SETTINGS.md`；引用本地快照文件名和位置是研究溯源，不表示将快照纳入发行。响应校验等尚未经实机证实的保守约束单独标注，不将其当成官网已保证的协议。

更正：上段历史“ATK Feature Report”用词应为普通HID Output路径；接口和请求证据以最新协议文档及本机验证为准。新增米家登录流程亦独立实现，不复制或分发官网下载JavaScript；云端验证码和二次验证仍须由用户按服务要求完成。

## 框架与直接依赖

应用使用 Tauri、Svelte、Vite、TypeScript，以及 Cargo/npm 声明的 HID、序列化、加密和 Windows API 依赖。准确解析版本由 `package-lock.json` 和 `src-tauri/Cargo.lock` 固定；锁文件应纳入版本管理。每个依赖保留其自身许可，不能推断它们全部采用 GPL。发布前应基于实际锁文件和安装包生成完整依赖归属清单，并检查许可证正文及源码提供方式。
