# 微光 Device Center 图标生成记录

日期：2026-10-23。生成模型请求值：`gpt-image-2.5`。使用现有本地 CPA 渠道 `http://127.0.0.1:8317/v1`；没有新增渠道、修改供应商配置或修改技能脚本。渠道实际上游模型身份未独立核验。

通过本地 imagegen 技能脚本调用 Images API。凭据仅通过子进程环境传入，未保存到素材目录。

## 成功产物

| 文件 | API | 参数 | 实际文件 |
| --- | --- | --- | --- |
| `app-original.png` | `/v1/images/generations` | `--model gpt-image-2.5 --size 1024x1024 --quality high --background transparent --output-format png --n 1 --no-augment` | 1254×1254 RGBA，alpha 范围 0–255，40.2 秒 |
| `tray-original.png` | `/v1/images/edits`，参考 `app-original.png` | 同上，使用 `edit --image assets/icons-v2/app-original.png` | 1254×1254 RGBA，alpha 范围 0–255，39.7 秒 |

原图按返回内容保留。渠道返回尺寸与请求尺寸不同，应用所需 1024 PNG、ICO 和托盘小尺寸 RGBA 由后续本地确定性适配产生。

完整原始提示词分别为 `app-prompt.txt` 和 `tray-prompt.txt`。应用图为深蓝青色与淡紫玻璃质感的开口光环及斜向光束；托盘图为同标志的白色简化版本，没有文字或设备堆砌。

## 检查与限制

已目视两张图形，并用 Pillow 核验 RGBA 及透明通道。托盘原图存在边缘碎屑和细描边，不能将原图直接当作最终小尺寸托盘；需要本地清理并核验 16–32px 显示效果。图像内容检查不等于 Windows 托盘实际运行验收。

共两张成功图像，未继续生成变体。首次执行在读取新版 CPA 配置结构时遇到本地 `KeyError: 0`，发生在网络调用前；识别 `access.api-keys` 后完成生成。没有因该预检失败消耗图像请求。没有进行任何硬件 IO。
