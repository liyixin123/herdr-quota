# 贡献指南

本文面向贡献者，说明开发前置条件、检查方式、Issue 信息要求与安全反馈渠道。用户安装与使用说明见 [README](README.md#安装与使用)。

## 开发环境

- Rust 1.89+（edition 2024；CI 固定 1.89.0）。
- just（`just check`、`just install`、`just uninstall` 的入口）。
- Herdr（本机验证版本 0.9.5）和已用 ChatGPT 订阅账号登录的 Codex CLI（`codex login`；本机验证版本 0.169.0）。
- 远程入口回归另需 Python 3.11+、jq，以及访问真实公开端点的网络；这只是开发与回归前置条件，普通安装不需要。

受支持平台为 macOS 15+ ARM64 与 Linux x86_64（基线见[发行说明](docs/releases.md)）。本地开发可在任一受支持平台进行；双平台构建与发布后验收由 CI 的 Release packages 工作流覆盖。

## 检查

提交前在仓库根目录运行：

```sh
just check
```

包含 `cargo fmt --all -- --check`、`cargo test --locked`（含隔离 HOME 与夹具 herdr/codex 的安装回归）与 `cargo clippy --locked --all-targets -- -D warnings`，与 CI 的 `fmt`/`test`/`clippy` 检查一一对应，见 [docs/ci.md](docs/ci.md)。

远程入口（`scripts/remote.sh`）回归：

```sh
cargo build --release --locked
python3 tests/remote/build_previous.py target/previous/herdr-quota
PREVIOUS_TEST_BINARY=target/previous/herdr-quota python3 -m unittest discover -s tests/remote
```

覆盖首次安装、重复安装、旧版更新、下载与校验失败保护、失败重试和卸载，包含访问真实公开端点的匿名下载回归。推送 `implement/**` 分支或发起 PR 会触发完整 CI。

## 提交 Issue

请使用 Issue 模板（仓库 Issue 页选择“问题反馈”）。为了能复现，请尽量提供：

- 系统与架构：`uname -sm` 的输出（受支持范围见[发行说明](docs/releases.md)）。
- Herdr 版本：`herdr --version`。
- Codex CLI 版本：`codex --version`。
- 安装方式：README 的远程安装命令 / 源码 `just install` / 其他（请说明）。
- 配置路径情况：是否设置了 `HERDR_CONFIG_PATH` 或自定义 XDG 路径。配置解析顺序为 `HERDR_CONFIG_PATH` → `$XDG_CONFIG_HOME/herdr/config.toml` → `~/.config/herdr/config.toml`；说明所用路径即可，请勿粘贴包含个人路径或账号信息的完整配置。
- 便于复现时：`herdr-quota status` / `refresh` 的输出（脱敏或示意即可），以及复现步骤。

请勿在 Issue 中包含 token、密码、账号邮箱、账号 ID 或会话内容。安全问题不要在公开 Issue 中报告，见 [SECURITY.md](SECURITY.md)。

## 安全问题

反馈渠道与报告时的脱敏要求见 [SECURITY.md](SECURITY.md)。

## 术语与边界

- 术语定义（如“当前账号”的含义）见 [GLOSSARY.md](GLOSSARY.md)。
- 额度缓存保存的内容与隐私边界见 [README](README.md#数据和失败状态)。
- 尚未验收的行为边界见 [docs/requirements.md](docs/requirements.md#待验证的两个核心问题)。
