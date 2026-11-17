# Herdr Quota

个人 Herdr TUI 订阅额度插件，项目暂名 `herdr-quota`。

已实现首期 Rust 状态命令，通过 Herdr 的 `ui.tab_bar_right.command` 显示；目前没有修改用户的 Herdr 配置。

## 首期目标

在 Herdr TUI 中快速查看当前 Codex 订阅的剩余额度：

- 5 小时窗口（5h）剩余百分比。
- 一周窗口（1w）剩余百分比。
- 全局可见，切换 workspace、tab 或 pane 后仍能查看，无需进入某个 Codex 会话。

文案示例（仅示意，非真实额度）：

```text
Codex 剩余 · 5h [████░░] 78% ↻2h14m · 1w [███░░░] 63% ↻3d08h
```

详细需求和待验证事项见 [docs/requirements.md](docs/requirements.md)。

## 构建和读取

需要 macOS / Linux、Rust 1.89+、已登录 ChatGPT 订阅账号的 Codex CLI。本机验证版本：Herdr 0.9.5、Codex CLI 0.169.0。

```sh
cd /Users/liyixin/repos/rust/herdr-quota
cargo build --release
./target/release/herdr-quota refresh --codex /opt/homebrew/bin/codex
./target/release/herdr-quota status --codex /opt/homebrew/bin/codex
```

`refresh` 在前台执行到期的读取，协议总超时 15 秒；60 秒内重复调用复用缓存。`status` 立即输出缓存，并在到期或窗口重置时启动一次后台读取。初次 `status` 可能显示“读取中”，下一次调用显示读取结果。

`--codex` 可指定本机 Codex 可执行文件的绝对路径，默认从 PATH 查找 `codex`。Herdr 的登录 shell PATH 可能不同，建议指定绝对路径。

## 接入 Herdr

生成包含当前二进制绝对路径的配置片段：

```sh
./target/release/herdr-quota herdr-config --codex /opt/homebrew/bin/codex
```

将输出字段合并到本机 `~/.config/herdr/config.toml` 的现有 `[ui]` 表中；已有 `tab_bar_right` 时将 command 项加入原数组，保留其他条目。然后执行 `herdr server reload-config`。配置生成命令只打印文本，不会覆盖文件或重载服务。需要回退时移除该 command 项并重载。

Herdr 每 5 秒执行一次本地 `status`，网络额度读取仍限每 60 秒一次；倒计时显示到分钟。状态显示与当前 pane 是否运行 Codex 无关，空间不足时 Herdr 会隐藏整个状态区。只支持本机 server。

## 数据和失败状态

- 每次刷新新建短生命周期 `codex app-server`，使用 Codex 自己管理的登录状态；不创建模型会话或启动 turn。
- 按窗口时长识别 5h / 1w，填充部分表示剩余；不将 API 余额或 token 统计当成订阅额度。
- 缺少窗口、非法百分比、未登录、非 ChatGPT 认证均显示未知或不可用；缺少重置时间显示“重置未知”。归零显示“待刷新”，不会自动补满额度。
- 同一可识别账号的额度请求失败时保留旧值并标记“过期”。账号无法验证、退出登录或检测到变化时清空旧额度，避免归属错误；跨进程账号变化在下次刷新时检测，不保证瞬时跟随。
- 缓存位于 `$XDG_CACHE_HOME/herdr-quota` 或 `~/.cache/herdr-quota`，可用 `--cache-dir` 指定绝对路径。只保存窗口数据、时间、状态及单向账号元数据摘要，不保存 email、原始账号 ID 或认证凭据。缓存目录和文件分别以 0700、0600 创建，写入为原子替换，进程锁避免重复请求。
- 不输出 Codex 的原始错误正文或 stderr。未知错误可通过 `refresh` 复查状态；删除专用缓存目录可重置读取状态。

真实账号读取已验证。真实账号切换、远程 server、无条件窄屏常驻不在已验证范围；接口事实与显示验收记录分别见 [Codex](docs/codex-readiness.md) 和 [Herdr](docs/herdr-readiness.md)。

## 开发检查

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```
