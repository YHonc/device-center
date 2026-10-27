# Windows 资源测量

`scripts/measure-resources.ps1` 只读采样指定 PID 及其子孙进程，不启动或终止应用，不修改系统设置。CSV 每行包含树汇总及 `processesJson` 中逐进程明细，JSON 保存统计、覆盖范围和限制。进程身份由 PID 和创建时间确定，避免 PID 复用造成 CPU 累计串接；已观察到的子进程在父进程退出后仍继续跟踪。

在 PowerShell 中执行，例如主应用 PID 为 12345：

```powershell
./scripts/measure-resources.ps1 -AppProcessId 12345 -DurationSeconds 600 -IntervalSeconds 2 -Scenario 'release-tray-stable'
```

默认采样 600 秒、间隔 2 秒；输出到 `artifacts/performance`。可用 `-OutputDirectory` 指定目录。必须填写真实应用 PID；脚本输出中的 `tenMinuteObservation` 只代表观测跨度达到十分钟，不是性能通过结论。首次启动、冷启动、热启动、主窗静止与托盘稳定常驻分别记录，不混合计算。

验收步骤：

1. 构建 release，手动启动应用，记录 PID、提交/版本、构建模式、Windows 版本、CPU 型号/逻辑核数、物理内存、WebView2 版本、设备数量和电源模式。
2. 将应用置于目标场景，等待状态稳定；托盘场景应关闭主窗并确认原生服务仍运行。每个稳定场景执行至少 600 秒采样。
3. 检查 `processesJson` 是否覆盖应用进程、WebView2 及已发现的子孙进程，核对进程树。使用外部复用进程、沙箱代理或从其他父进程启动的服务时，须补充独立采样；它们不能自动归属于指定 PID 树。
4. 核对 JSON 的观测时长、CPU 覆盖秒数、不完整区间和警告。缺失数据不能视为零，也不能直接作为通过依据。树退出会提前结束。
5. 托盘预算暂为全树私有工作集不超过 80 MiB、平均 CPU 小于 0.5%；主窗静止预算为 180 MiB、平均 CPU 小于 1%。同时报告均值和峰值及原始 CSV；预算仍待实测，不以本工具的存在宣称通过。原规划写 MB，本工具明确用 MiB（1,048,576 bytes），报告必须保留单位。

CPU 计算为相邻样本的各进程累计处理器时间增量之和，除以实际墙钟间隔和逻辑核数后乘 100，因此 100% 表示所有逻辑核满载。新进程若确定在本区间内创建，可计入其从创建以来的 CPU；没有基线的旧进程、退出或读取失败的进程会让该 CPU 区间标记不完整，不参与平均和峰值。均值按有效区间时长加权。第一条样本只建立基线。

私有工作集通过 `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate` 按 PID 读取，代表当前驻留的私有页面，不是总工作集或私有提交内存。内存均值按样本计算；系统性能计数器可能有更新延迟。轮询无法捕获在两个样本之间创建并退出的短命进程，采样本身也消耗资源；不得把缺失或有效区间偏少的统计描述为完整性能验收。

## 已记录的用户会话采样

2026-10-23记录 `artifacts/performance/20261023T043643455Z-summary.json`，场景为 `portable-auto-user-session-window-state-unverified`，根PID54764，Windows10.0.22631.0、20逻辑核。文档代理已核对原始JSON：

| 指标 | 结果 |
|---|---|
| 观察跨度 | 602.2473593秒，duration_reached，tenMinuteObservation=true |
| 样本/CPU区间 | 88样本，87有效CPU区间，首条基线不完整CPU样本1，内存不完整0 |
| CPU（按20逻辑核归一） | 均值0.01881%，峰值0.63871% |
| 全进程树PrivateWorkingSet | 均值101.691MiB，峰值132.270MiB |
| warnings | 空数组 |

这是真实用户会话的十分钟跨度采样；窗口状态未验证，不能称为纯前台静止或托盘稳定常驻，也不能据此声称托盘≤80MiB达标。纯前台/托盘分场景预算、窗口关闭后功能、WebView释放、休眠恢复及设备状态变化仍待独立验收。原始CSV与JSON应共同保留，采样方法和盲区见上文。

此前 `20261023T035920305Z-summary.json` 仅363.6737931秒，进程退出且期间含扫码，仍不能作为十分钟idle证据。本次文档结果同步晚于已生成的源码zip归档；未为此重打归档，归档内性能说明可能未包含本次最终结果。
