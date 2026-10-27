# 当前交付

最新预设交互修订：`releases/portable-presets/DeviceCenter.exe`，构建标识presets-20261024；ZIP为`releases/DeviceCenter-presets-20261024.zip`，源码为`releases/source/DeviceCenter-source-presets-20261024.zip`。EXE SHA256：`D07E3E27593033DCDC612E290DDAC8B4BB8FD8E44AAB48BABA8AC099DD0F672A`。前端38项通过，前端构建0错误/0警告；release成功，Rust仍有一条未使用辅助函数警告。原位重命名替代齿轮管理弹窗，旧包保留。

## 最新：glass-20261024

入口：`releases/portable-glass/DeviceCenter.exe`；压缩包：`releases/DeviceCenter-glass-20261024.zip`；源码归档：`releases/source/DeviceCenter-source-glass-20261024.zip`。

EXE SHA256：`AE0A0142E4DDD52A1298100518E72736FDB5299846C9194B7EDA7858EFA5F43D`。release构建成功（前端0错误/0警告，Rust一条未使用辅助函数警告），前端36项与Rust90项通过。包括KX键表读写长度修复、设置直达和官方链接、移除固件、预设即时应用/右侧保存及统一材质。新包尚未启动，实机只读与待写入验收边界见ACCEPTANCE；下方保留此前交付信息。

更新：2026-10-24。官网设置对齐调试程序已release构建并复制至`releases/portable-official/DeviceCenter.exe`；版本仍为0.1.0。历史安装包仍是原版本。源码实现、离线检查和设备功能验收分别记录。

最新调试程序包含ATK六项性能/19项动作、KX75基础性能/官方动作目录/单色RGB，以及设备设置页面重整。EXE SHA256为`DF952A5684874E2D78D7FAC5ED50CE2780F08CD6241F9FD2D88791E2973B7561`。发布归档名称为`releases/DeviceCenter-official-20261024.zip`，对应源码为`releases/source/DeviceCenter-source-official-20261024.zip`及同名manifest.json；归档由主代理在本记录保存后生成。下方extensions包及哈希属于此前版本。

最新调试入口：`releases/portable-debug/DeviceCenter.exe`；压缩包：`releases/DeviceCenter-extensions-20261023.zip`。构建标识为0.1.0 / extensions-20261023。EXE SHA256：`19BB4ABBAC65313DA378798176FC51F9D21129A2099B14657A63C23F7392A9C9`。对应源码及逐文件清单位于`releases/source/DeviceCenter-source-extensions-20261023.zip`及同名manifest.json。

本次release构建成功，前端0错误/0警告；Rust有一条旧映射辅助函数未使用警告。便携包保留WebView2Loader及许可文件，ZIP内容已核对。尚未启动此调试程序或进行硬件功能验收。下述未出包描述属于打包前的源码验证阶段。

| 文件 | 用途 |
|---|---|
| releases/portable-official/DeviceCenter.exe | 最新官网设置对齐调试程序 |
| releases/portable-polish/DeviceCenter.exe | 历史polish界面基线 |
| releases/DeviceCenter-portable-polish.zip | polish便携压缩包 |
| releases/DeviceCenter_0.1.0_x64-setup.exe | 先前custom安装包，不包含后续polish的保证 |
| releases/source/ | 各历史源码归档及manifest |
| releases/portable-* | 历史阶段便携目录 |

便携版保留完整目录的DLL与许可，切换时先退出旧版托盘。当前程序依赖系统WebView2 Runtime。

polish历史记录：前端构建、50项Rust测试、15项Node测试通过，两个硬件/网络测试忽略；浏览器检查过快捷窗和诊断入口。该轮新程序未启动，桌面与实机待验项见 ACCEPTANCE.md。本次整理的检查记录见 REORGANIZATION.md。

新发行应记录构建版本、文件哈希、对应源码和验收范围。旧版哈希与构建过程保存在 history，不作为新文件证明。

本轮交付范围为可编辑源码、协议说明和更新后的开发顺序。宏编辑/设备写入及绑定、内置灯效与固件官网入口的边界见 SETTINGS.md。统一离线检查结果见 ACCEPTANCE.md；真实设备功能、桌面行为、安装与性能实测放在最后，未用历史证据替代。

此前扩展出包前的离线检查：前端测试23/23、构建0错误/0警告，Rust测试65 passed、0 failed、2 ignored。宏录制已按动作后延迟处理，异步导入增加跨设备结果门控，放弃/关闭/卸载时清理草稿。未重新生成发行包，未运行ignored硬件/网络探针，未启动新桌面程序；这些结果证明离线构建与测试通过，未证明真实设备写入或固件兼容。

官网设置对齐轮最终检查：前端32/32、构建0错误/0警告，Rust82通过、0失败、3忽略。设置fixture预览见`artifacts/official-settings-20261023/local-kx-settings.png`和`local-atk-settings.png`，不包含设备IO。KX75授权只读探针在身份查询阶段500ms及2200ms两次超时，尚未到GET44/49且无SET；读取故障尚未解决，等待设备唤醒及具体错误后复验，未证实失败根因。完整官网功能及真实硬件行为尚未验收，详见ACCEPTANCE.md。

后续唤醒设备后只读调试已推进：ATK六个GET约4ms成功，确认CID2/MID36、1000Hz、零基档位1/4及DPI400/800/1600/3200；KX身份和GET44读取128字节约44ms成功。GET49仍因回复包数不等于37而失败，颜色捕获修正中。ATK超时提示已说明可能休眠、移动鼠标后重读，不自动唤醒或重试。尚无SET和写入功能验收，发行包未更新。

2026-10-24最终回归与构建：KX GET49已根据35帧真机捕获修正为490字节配置区，SET仍为518字节运输区；Rust85通过、0失败、3忽略，前端测试32/32、release前端检查0错误/0警告。release约1分27秒完成，有一条旧映射辅助`apply`未使用警告。键盘再次唤醒后GET44 128字节约51ms、GET49 35包490字节约147ms成功。上述只有只读设备证据，无SET或实际映射/RGB写入验收。新程序尚未启动，启动前退出旧托盘并保持设备唤醒，便携目录中DLL与许可文件应一并保留。
