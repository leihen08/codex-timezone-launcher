# ADR-002：通过包调试环境激活 Microsoft Store 客户端

## 状态

已接受

## 日期

2026-09-07

## 背景

OpenAI Codex 的 Microsoft Store 包把主程序放在受 AppModel 管理的 `WindowsApps` 目录。直接对包内 `ChatGPT.exe` 调用 `CreateProcess` 会失败；正确的 AUMID 激活可以启动应用，但 Windows 不会把调用者的每次启动环境变量传给被激活进程。因此仅改用 `IApplicationActivationManager` 仍无法满足进程级 `TZ` 要求。

## 决策

发现层将客户端建模为普通 EXE 或 Store 包目标。Store 目标包含展示用 EXE 路径、包全名和 AUMID。

启动 Store 目标时：

1. 创建当前用户命名空间中的一次性握手事件。
2. 通过 `IPackageDebugSettings::EnableDebugging` 注册只含 `TZ` 的双 NUL UTF-16 环境块，并指定当前启动器 EXE 的隐藏恢复模式。
3. 通过 `IApplicationActivationManager::ActivateApplication` 激活 AUMID。
4. Windows 以 `-p`/`-tid` 参数调用恢复模式；恢复器只打开该线程、调用一次 `ResumeThread` 并发出握手事件。
5. 主进程收到事件后立即调用 `DisableDebugging`。所有错误路径也通过守卫尝试撤销设置。

隐藏恢复模式在程序入口最先识别并执行，早于交互式启动器的单实例锁，因此父窗口已运行时也不会阻止恢复线程。

启动器不定义或调用包终止方法。若客户端已运行，工作流在启用包环境之前返回警告。

## 考虑过的替代方案

### 直接启动 WindowsApps 内部 EXE

- 被 AppModel 拒绝，正是 1.0.0 报错的根因。

### 仅使用 AUMID 激活

- 能正确启动 Store 应用，但无法传递每次启动的 `TZ`。

### 修改用户或系统环境变量

- 会影响非目标进程并产生竞态，违背仅对子进程生效的要求。

### 复制完整 Store 包到普通目录

- 当前包接近 2 GB，并可能失去包身份、授权和自动更新语义。

## 后果

- Store 版可以获得经验证的进程级 `TZ`，普通安装版仍走参数化 `CreateProcess`。
- Store 激活最多等待握手 10 秒，所以整个工作流在后台线程执行。
- 启动期间暂时禁用窗口关闭；这样正常退出不会跳过调试设置清理。若进程异常崩溃，下次激活前会先尝试清理旧设置。
- 该方案使用 Windows 官方诊断接口；实现必须持续保留事件前缀校验、数值线程 ID 校验和无终止能力约束。
