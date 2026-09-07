# Codex 时区启动器

一个干净、原生的 Windows 启动器。它在启动 Codex/ChatGPT 桌面客户端时，仅向新子进程注入经过校验的 IANA `TZ` 环境变量；不会修改 Windows 系统时区，也不会结束任何现有进程。

## 快速使用

1. 关闭正在运行的 Codex/ChatGPT 客户端；请先保存未完成内容。
2. 运行 `outputs\CodexTimeZoneLauncher.exe`。
3. 选择 IANA 时区，或点击“自动定位”。
4. 点击“保存并启动 Codex”。选择会保存到固定位置：

   `%LOCALAPPDATA%\ChatGPTTimeZoneLauncher\settings.json`

如果检测到客户端仍在运行，启动器会提示自行退出，但绝不会强制结束进程。时区环境变量是在进程启动时读取的，所以已经运行的客户端不会即时切换。

此本地构建没有商业代码签名证书，Windows 可能显示“未知发布者”；可用随附的 `SHA256SUMS.txt` 核对文件完整性。

## 适用范围与限制

- 适用于 ChatGPT/Electron 使用运行时默认时区的日期显示、格式化和相关逻辑。
- 极少数直接调用 Windows 原生时区 API 的功能仍可能使用系统时区。
- 启动器使用完整 IANA 时区表校验选择，例如 `Asia/Shanghai`、`America/New_York`。
- 默认值是中性的 `Etc/UTC`；首次保存后会自动恢复上次选择。
- 可读取旧版同目录设置中的 `TimeZoneName`，但会忽略任何旧的自动重启策略；下次保存时迁移为新格式。

## 自动发现客户端

发现顺序包括：正在运行的桌面客户端路径、当前用户的 AppModel 包注册（Microsoft Store 版）、Windows App Paths，以及常见的用户级和机器级安装目录。仅接受实际存在且文件名为 `ChatGPT.exe` 或 `Codex.exe` 的目标。

启动使用 Rust `std::process::Command`（底层 Windows 进程 API）和独立环境映射，不拼接或执行 shell 命令。对于 `Codex.exe`，运行检测还要求路径与目标一致，从而避免把 Codex CLI 误判成桌面客户端。

## 自动定位与隐私

“自动定位”只请求固定端点 `https://ipapi.co/json/`：

- 只使用响应中的 `timezone` 字段；
- 不在设置、日志或界面状态中保存公网 IP；
- 禁止 HTTP 重定向，使用系统 TLS 证书校验；
- DNS/连接/发送/接收均有超时，响应上限为 64 KiB；
- 离线、超时、HTTP 错误、无效 JSON 和无效时区都有清晰提示。

## 从源码构建

要求：

- Windows 10/11 x64；
- Rust 1.98.1（由 `rust-toolchain.toml` 锁定）；
- Visual Studio 2022 Build Tools，包含 `Desktop development with C++`/Windows SDK。

可用官方包管理器安装构建依赖：

```powershell
winget install --id Rustlang.Rustup --exact
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

在 Developer PowerShell 或能够找到 MSVC 工具链的 PowerShell 中运行：

```powershell
./scripts/build-release.ps1
```

脚本依次执行格式检查、Clippy（警告视为错误）、全部测试、`--locked` Release 构建，然后生成：

- `outputs\CodexTimeZoneLauncher.exe`
- `outputs\SHA256SUMS.txt`

## 开发命令

```powershell
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

测试覆盖时区校验、设置记忆和旧格式迁移、固定设置路径、进程运行提示决策、CLI 误报规避、安装路径发现、子进程环境隔离、IP 响应解析及离线/超时错误映射。本机联调还会验证 Store 版客户端真实发现和当前运行实例检测。

## 结构

- `src/config.rs`：固定路径设置与原子写入。
- `src/discovery.rs`：客户端路径发现。
- `src/process.rs`：运行检测与无 shell 启动。
- `src/geo.rs`：固定 HTTPS 定位。
- `src/workflow.rs`：可测试的保存/启动顺序。
- `src/ui*.rs`：DPI 感知 Win32 界面与事件处理。
- `docs/decisions/0001-native-process-local-timezone.md`：关键设计取舍。
