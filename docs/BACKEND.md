# 原生后端

## 服务边界

`src-tauri/src/lib.rs` 注册类型化 Tauri commands，协调窗口、托盘、HID 和灯服务。设备操作按 deviceId 使用独立队列，同设备读取、配置冲突检查、写入、读回共用一把锁。主窗关闭销毁 WebView，重开后恢复状态；后台服务独立运行。

| 入口 | 用途 |
|---|---|
| list_hid_devices / read_battery | 只读枚举和电量 |
| read_device_settings / apply_device_settings | 基础设置 |
| read_button_mappings / apply_button_mapping | 鼠标功能和有限快捷键 |
| read_keyboard_mappings / apply_keyboard_mapping | KX75分层映射 |
| get_lamp_config / save_lamp_config | 本地灯配置 |
| read_lamp / set_lamp | 灯状态及写后确认 |
| begin/poll/list/import/cancel 米家命令 | 见 CLOUD.md |

完整注册以 lib.rs 的生产 generate_handler 为准。

## 设备与协议

- ATK：373B:101A、FF02、usage 2、interface 1，Output报告查询；接收器身份不用于推断所有F1子型号。
- KX75：3554:FA09、FF02、usage 2、interface 1；每次事务核验型号签名 `03 00 00 00 01 C7`。充电语义保持未知。
- 挂灯：固定 UDP 54321；legacy get_prop优先，符合条件时走有型号门控的MIoT。亮度1–100，色温2700–6500。ACK后读回目标值。
- 凭据：当前Windows账户DPAPI加密，界面只取configured/IP；不返回保存的Token。

设置块、协议依据和未闭合字段见 SETTINGS.md；云契约见 CLOUD.md，轮询见 MONITOR.md。历史实机结果与待验项统一见 ACCEPTANCE.md。

## 构建

使用 `src-tauri/build-windows.ps1` 的 test/check/release/installer入口。GNU工具链在 `.tools`；脚本临时映射ASCII盘符，并将target放在 `%TEMP%/device-center-target-<工作区哈希>`。构建期间不并发运行同一脚本。

发行文件归入 `releases/`。源码归档由 archive-source.ps1白名单收集，排除研究材料、工具链和运行数据。
