# Herdr Quota

个人 Herdr TUI 订阅额度插件，项目暂名 `herdr-quota`。

通过 Herdr 的 `ui.tab_bar_right.command` 显示额度。安装后，将生成的配置片段手动合并到 Herdr 配置即可启用。

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

## 安装与使用

### 1. 检查前置条件

需要 macOS / Linux、Git、Rust 1.89+（含 Cargo）、Herdr，以及已登录 ChatGPT 订阅账号的 Codex CLI。本机验证版本为 Herdr 0.9.5、Codex CLI 0.169.0。

在终端中检查这些命令可用。

```sh
git --version
rustc --version
cargo --version
herdr --version
codex --version
codex login status
```

若尚未登录，执行 `codex login` 并使用 ChatGPT 订阅账号完成登录。

### 2. 从源码安装

```sh
git clone https://github.com/liyixin123/herdr-quota.git
cd herdr-quota
cargo install --path . --locked --root "$HOME/.local"
"$HOME/.local/bin/herdr-quota" --version
```

已有源码时，在仓库根目录执行 `cargo install` 及之后的命令即可。以上安装位置为 `~/.local/bin/herdr-quota`，后续均使用绝对路径。

如果希望在终端直接运行 `herdr-quota`，将下面这一行加入所用 shell 的配置文件（如 `~/.zshrc` 或 `~/.bashrc`），再重新打开终端。

```sh
export PATH="$HOME/.local/bin:$PATH"
```

### 3. 读取一次额度

先运行 `command -v codex`，取得 Codex 可执行文件的绝对路径。将下面的 `/absolute/path/to/codex` 替换为该路径，例如 macOS Homebrew 安装可能为 `/opt/homebrew/bin/codex`。

```sh
command -v codex
"$HOME/.local/bin/herdr-quota" refresh --codex /absolute/path/to/codex
"$HOME/.local/bin/herdr-quota" status --codex /absolute/path/to/codex
```

成功时会输出类似前面的额度示例。`refresh` 在前台执行到期的读取，协议总超时 15 秒；60 秒内重复调用复用缓存。`status` 立即输出缓存，并在到期或窗口重置时启动一次后台读取。初次 `status` 可能显示“读取中”，下一次调用显示读取结果。

`--codex` 默认从 PATH 查找 `codex`。Herdr 的登录 shell PATH 可能不同，接入时建议指定绝对路径。

### 4. 接入 Herdr

使用已安装的程序生成包含其绝对路径的配置片段，同样替换 Codex 路径。

```sh
"$HOME/.local/bin/herdr-quota" herdr-config --codex /absolute/path/to/codex
```

打开本机 `~/.config/herdr/config.toml`，将输出字段合并到现有 `[ui]` 表中。使用自定义 Herdr 配置路径时，编辑实际使用的文件。

- 没有 `[ui]` 时，添加整个输出片段。
- 已有 `[ui]` 时，将字段合并进该表，避免重复声明 `[ui]`。
- 已有 `tab_bar_right` 时，将生成的 command 项加入原数组，保留其他条目。
- 设置 `hide_tab_bar_when_single_tab = false` 和 `tab_bar_position = "top"`，以便单 tab 时也显示顶部状态栏。

保存后，在正在使用的本机 Herdr 会话的终端中执行以下命令。

```sh
herdr server reload-config
```

等待约 5 秒，检查顶部 tab 行右侧是否出现 `Codex 剩余`。配置生成命令只打印文本，配置合并和重载需要按上述步骤执行。

Herdr 每 5 秒执行一次本地 `status`，网络额度读取仍限每 60 秒一次；倒计时显示到分钟。状态显示与当前 pane 是否运行 Codex 无关，空间不足时 Herdr 会隐藏整个状态区。只支持本机 server。

### 更新与卸载

更新时，在源码仓库中拉取最新代码并重新安装。

```sh
git pull --ff-only
cargo install --path . --locked --root "$HOME/.local" --force
```

安装位置保持相同时，Herdr 下次调用会使用更新后的程序。

卸载时，先从 Herdr 的 `tab_bar_right` 数组中移除对应 command 项，保存并执行 `herdr server reload-config`，然后卸载程序。

```sh
cargo uninstall --root "$HOME/.local" herdr-quota
```

### 常见问题

- 终端显示“未登录”或“非订阅账号”时，执行 `codex login status`，确认 CLI 使用 ChatGPT 订阅账号。
- 显示“读取失败”时，确认 `--codex` 指向可执行文件，并在终端运行上面的 `refresh` 命令复查。刚尝试过读取时，等待 60 秒后重试。
- 终端能显示额度、Herdr 中没有显示时，检查配置中的程序路径与 Codex 路径、是否已重载配置，以及 tab 行是否有足够空间。
- 显示“过期”时，程序保留了同一账号的旧额度，后续到期刷新成功后会更新。

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
