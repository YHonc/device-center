# 当前交付

文档核对：2026-10-27。当前源码与已打包程序分开记录。

## 当前源码

包含10月24日设备功能、灯光预设交互及10月25日UI与编辑交互收敛。10月25日记录为44项前端测试通过、构建0错误0警告；未重打包。详见 [ACCEPTANCE.md](ACCEPTANCE.md)。

## 最近已记录的便携交付

| 项目 | 路径或标识 |
|---|---|
| 构建标识 | presets-20261024 |
| 程序 | releases/portable-presets/DeviceCenter.exe |
| 便携ZIP | releases/DeviceCenter-presets-20261024.zip |
| 对应源码快照 | releases/source/DeviceCenter-source-presets-20261024.zip |
| EXE SHA256 | D07E3E27593033DCDC612E290DDAC8B4BB8FD8E44AAB48BABA8AC099DD0F672A |

本次重新计算EXE哈希，与上述历史记录一致。该包的历史记录为38项前端测试通过、前端构建0错误0警告、release成功，Rust有一条未使用辅助函数警告。它包含预设原位重命名，不包含随后10月25日源码变更的交付证明。

使用时保留便携目录中的DLL与许可文件，并先退出旧版托盘。系统需要WebView2 Runtime。

## 历史版本

| 版本 | 保存位置 |
|---|---|
| glass-20261024 | releases/portable-glass/ |
| official-20261024 | releases/portable-official/ |
| extensions-20261023 | releases/portable-debug/ |
| polish及更早阶段 | releases/portable-polish/及其他portable目录 |
| 历史安装包 | releases/DeviceCenter_0.1.0_x64-setup.exe |

各版构建、哈希和只读调试过程保存在 [历史交付记录](history/DELIVERY_BEFORE_SYNC_2026-10-27.md)。安装包不自动跟随源码或便携包更新。

后续出包时记录构建标识、文件哈希、对应源码和实际验证范围。当前实机写入、桌面系统行为与性能待验项统一见验收矩阵。
