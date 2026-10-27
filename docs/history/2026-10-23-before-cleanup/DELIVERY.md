# 当前交付与验收边界

日期：2026-10-23。本文件汇总当前可运行文件及已有证据，不表示全部功能完成。它更新早期草稿中的构建和UI状态；旧性能、设备和登录证据仍须按各自覆盖范围理解。

## 可运行文件

| 文件 | SHA256（文档代理现场校验） |
|---|---|
| `src-tauri/portable-custom/DeviceCenter.exe` | `1D6D233D8EA2353198CCFE814A9EBF241C8353167A804061304EC2BCCEBD4FB7` |
| `src-tauri/DeviceCenter_0.1.0_x64-setup.exe` | `D5A0724ECA82A3510A9E2C85103C41ABDBAD0F2571672401E04E7C7D7DEDF962` |
| `src-tauri/DeviceCenter-portable-custom-0.1.0.zip` | `6AF1AC2E90116B8D3FA1A50032EAECBCAAC5AA63A694DE0C52F5F5DA8B1350F4` |

便携版从项目根执行：

```powershell
& './src-tauri/portable-custom/DeviceCenter.exe'
```

复制便携版时保留整个 `portable-custom` 目录，包括WebView2Loader.dll、LICENSE、LICENSE-ATK、UPSTREAM.md和第三方说明；不要只复制exe。系统仍需要WebView2 Runtime。本轮用户设备休眠，新包保持未启动，不执行安装/卸载、唤醒、硬件或云端IO。上面命令供用户后续自行启动，不是本轮已执行记录。

主代理已于12:36:22切换启动portable-auto，报告PID54764；此前portable-next的PID52984仅为旧版记录。进程号为历史快照；本文件未独立操作Windows窗口按钮。

## 已有验证

- 本轮原生测试 `.tools/custom-final-test.log:60` 为 **50 passed、0 failed、2 ignored**，node `.tools/custom-node-test.log` 为15通过、0失败；主代理样本键盘预览/应用AX检查完成。本轮用户设备休眠，无实机IO、唤醒或正常启动，mock结果不证明真实SET。
- 两个交付文件的SHA256与上表一致；文件存在与测试通过分别记录，不将其合并为完整功能验收。
- 自动选择逻辑4项测试通过（主代理结果）；portable-auto自动读取已有用户成功反馈，不扩大为SET或实际开关变化通过。
- 前轮complete的 `.tools/install-validation-complete.log` 有UPDATE保护模式安装/卸载退出0及配置/原进程不变；本轮custom不执行安装卸载，只做NSIS静态payload核验。前轮检查不能代替新包安装或日常普通卸载验收。
- 主代理报告新版真实窗口内容检查为1280×720，main client/scroll高度均672，无主内容滚动，并完成抽屉及连接模态检查。该轮1280×720检查本身不覆盖目标尺寸及全部缩放组合；后续1040×720浏览器证据见下一条。
- 后续主代理在浏览器1040×720检查main client/scroll均672，首页无溢出，截图可见完整按钮。该证据覆盖浏览器目标尺寸布局，仍不替代Windows窗口按钮点击和全部系统缩放验收。
- 用户随后反馈“已自动导入且读取成功”，经追问确认仍使用portable-next旧窗口。因此portable-next米家登录、用户导入及灯状态读取已有成功反馈，不能据此认定portable-auto自动选择实测通过，也不证明灯光设置写入。
- 主流程此前在本机ATK FF02只读取得电量95%、1000Hz、已有DPI第2档/共4档；不证明写入及其他固件。KX75 FF02型号probe成功，电量与性能GET的最终结果仍需后端证据。

## 实现范围与剩余

源码包含三设备dashboard、本地键鼠能力设置、挂灯miIO、后台轮询与退避、低电量提醒、托盘、可选自启和应用内米家配对。源码存在不代表每项系统行为或实机功能已经通过。

| 项目 | 当前结论与下一步 |
|---|---|
| 新版视觉 | 已有真实窗口内容、抽屉与连接模态检查；边框按钮、拖动、贴靠与完整键盘/缩放/减少动态效果仍待直接验收 |
| 米家配对 | portable-next登录、用户导入及灯状态读取有用户成功反馈；切换portable-auto PID54764后用户确认“可以 现在能自动读取设备了”，新版自动读取记为用户确认成功。此原话不细分扫码目标选择过程，不证明设置写入或开关实际变化 |
| 键鼠配置 | ATK读取有本机证据；设置SET未验证，全块CAS/失败恢复仍须实机；不凭型号probe认定KX75电量/性能成功 |
| 通知、自启和生命周期 | snapshot每设备锁、独立灯原生轮询、360×440 compact与主窗销毁重建已实现；隔离无硬件/云网络的生命周期闭环通过，真实通知、自启、休眠恢复及SET仍待验 |
| 安装卸载 | UPDATE保护模式隔离安装/卸载均退出0且配置/原进程未变；普通卸载、新用户环境、WebView2引导仍待验 |
| 性能 | 新用户会话样本20261023T043643455Z已完成602.247秒/88样本/87有效CPU区间：20核归一CPU均0.01881%峰0.63871%，树PrivateWorkingSet均101.691MiB峰132.270MiB，无warnings。窗口状态未验证，不证明纯前台或托盘≤80达标；前台/托盘分场景仍待验 |
| 发行许可 | GPL正文、MIT通知及来源说明已有；最终依赖完整清单与发行包内容仍需核对。官网下载JavaScript不分发 |

详细验收矩阵见 `docs/ACCEPTANCE.md`，新版视觉目标见 `docs/UI_SPEC.md`，协议与云端边界见相关后端文档。后续如替换二进制，应重新记录哈希、构建/测试日志和对应验收，不能继续引用本次哈希作为新文件证据。

本轮新增生成图标（原imagegen CLI/Images API请求gpt-image-2.5成功两张，记录在assets/icons-v2/GENERATION.md）、ICO与小尺寸托盘适配；托盘logo/mouse/keyboard/both四模式仅缓存/变化重绘，含数字?/!和tooltip。ATK6按钮12功能与最多2modifier+A–Z/数字组合、KX75 Default/Fn/Fn2普通字母数字及任务管理器/安全选项预设已有实现，任意自定义组合未证实不开放。真实SET、实际按键、充电语义、自启重登及长期托盘/通知仍待验；产物名称不表示全部实测通过。

隔离生命周期持久证据位于 `docs/evidence/lifecycle-complete-{events.jsonl,summary.json,resources.json}`。已核对短OS样本：主窗7进程/6 WebView、179.95MiB；零窗20秒1进程/0 WebView、5.62MiB，第二轮5.82MiB。无真实硬件/云网络参与，不能扩大为SET或十分钟托盘预算通过。最新用户会话性能方法与结果见 `docs/PERFORMANCE.md`；最终源码归档由backend在本轮文档冻结后生成。
