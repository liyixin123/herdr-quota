# Herdr Quota

个人 Herdr TUI 订阅额度插件，项目暂名 `herdr-quota`。

通过 Herdr 的 `ui.tab_bar_right.command` 显示额度。远程安装会自动备份并合并配置、重载本机 Herdr。

## 首期目标

在 Herdr TUI 中快速查看当前 Codex 订阅的剩余额度：

- 5 小时窗口（5h）剩余百分比。
- 一周窗口（1w）剩余百分比。
- 全局可见，切换 workspace、tab 或 pane 后仍能查看，无需进入某个 Codex 会话。

文案示例（仅示意，非真实额度）：

```text
Codex 剩余 │ 5h [████░░] 78% ↻2h10m │ 1w [███░░░] 63% ↻3d07h
```

实际 tab 栏显示效果（示意数据：测试夹具账号，按实际程序输出逐字符渲染；不含真实账号、路径或会话内容）：

![Herdr tab 栏额度显示（示意数据）](docs/images/quota-display.png)

使用竖线区分额度窗口，异常状态以方括号标记，例如 `[过期]`、`[读取失败]`。Herdr 0.9.5 会剥离命令输出的 ANSI 颜色，因此这里使用纯文本分隔。

详细需求和待验证事项见 [docs/requirements.md](docs/requirements.md)。

## 安装与使用

支持 **macOS 15+ ARM64** 和 **Linux x86_64**（系统基线见[发行说明](docs/releases.md)）。先安装并启动 Herdr，安装 Codex CLI 并执行 `codex login`，使用 ChatGPT 订阅账号登录。本机验证版本为 Herdr 0.9.5、Codex CLI 0.169.0。

安装命令只需要常见系统工具：`sh`、`curl`、`tar`、`uname`、`mktemp`、`awk`、`chmod`、`rm`，以及 `sha256sum` 或 `shasum`。仓库与发行包均为公开，普通安装无需 Git、Rust、just、Python、gh，也无需 GitHub token 或 `jq`。只有在私有镜像或 fork 上安装时，才需要设置 `GH_TOKEN` 或 `GITHUB_TOKEN`（细粒度 token 需要对应仓库的 Contents read 权限）并安装 `jq` 走认证下载，机制见[安装管理](docs/install-management.md)。

### 一条命令安装、重试或更新

入口脚本固定到 `v0.1.2` tag，程序本身每次选择最新 Release，因此重跑同一命令即可安装、更新或修复未完成的安装：

```sh
curl -fsSL https://raw.githubusercontent.com/liyixin123/herdr-quota/v0.1.2/scripts/remote.sh | sh -s install
```

`| sh` 在这里是安全的：脚本不读标准输入，`install` 是它的最后一步，前面依次完成 HTTPS 下载、SHA256 校验、归档结构检查（只允许根目录单个 `herdr-quota` 普通文件）和 `--version` 与 tag 一致性核对。下载被截断只会让它在校验前报错退出，不可能跳过校验去安装。

成功时程序安装到 `~/.local/bin/herdr-quota`，自动定位 Codex 的绝对路径、验证订阅登录，在修改配置前创建 0600 权限的备份并打印路径，然后重载 Herdr。配置默认使用 `HERDR_CONFIG_PATH`、`$XDG_CONFIG_HOME/herdr/config.toml` 或 `~/.config/herdr/config.toml`。无需将安装目录加入 PATH；配置使用绝对路径。保留已有 tab 栏位置、其他状态项及注释，并开启单 tab 时显示 tab 栏。

等待约 5 秒，在 Herdr 的 tab 行右侧应看到 `Codex 剩余` 和 5h / 1w 百分比、重置倒计时。空间不足时 Herdr 会隐藏状态区。状态与当前 pane 是否运行 Codex 无关，只支持本机 server。

同一命令支持重复安装、更新与失败后重试，不会添加重复配置。下载、校验或发行包验证失败时保留现有程序。Herdr 未启动或重载失败时，修复后重跑同一命令；已安装程序和已保存配置会保留用于恢复。配置解析失败不会覆盖原文件。

### 一条命令卸载

将上面安装命令末尾的 `install` 换成 `uninstall`，即为卸载命令：

```sh
curl -fsSL https://raw.githubusercontent.com/liyixin123/herdr-quota/v0.1.2/scripts/remote.sh | sh -s uninstall
```

卸载同样下载并验证临时清理程序，因此即使已安装程序被手动删除也可清理；不需要 Codex 或登录信息。只移除本程序管理的额度项和安装文件，保留其他配置、备份和额度缓存；不会用旧备份覆盖用户后续修改。失败后修复并重跑，支持重复卸载。详细恢复规则、路径参数见[安装管理](docs/install-management.md)。

### 本地开发与维护

在源码仓库中准备 Rust 1.89+、just、Herdr 和已登录的 Codex，然后执行：

```sh
just install
just uninstall
just check
```

本地命令编译后复用相同的原生配置管理行为。需要手动复查额度时：

```sh
"$HOME/.local/bin/herdr-quota" refresh --codex /absolute/path/to/codex
"$HOME/.local/bin/herdr-quota" status --codex /absolute/path/to/codex
```

`refresh` 在前台读取，单次会话总预算 30 秒（含冷启动 `codex app-server` 的开销）；60 秒内复用缓存。`status` 立即输出缓存，到期或额度窗口重置后启动后台读取，初次可能显示“读取中”。Herdr 每 5 秒调用一次本地 `status`，网络额度读取仍限每 60 秒一次。`--codex` 默认从 PATH 查找；自动安装会将发现的绝对路径写入配置。

### 常见问题

- 下载失败时，检查网络、安装命令中固定的 bootstrap tag 与最新 Release 是否已发布。SHA256 不符时不要绕过校验，稍后重跑或联系维护者。
- 不支持的平台需要自行从源码构建；缺少工具时按提示安装后重跑。Herdr 重载失败时先启动本机 Herdr；安装提示失败时不要仅凭文件存在判断完成。
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
just check
```

远程入口回归命令、开发前置条件与完整贡献流程见[贡献指南](CONTRIBUTING.md)；CI 检查与仓库保护见 [docs/ci.md](docs/ci.md)。

## 贡献与反馈

贡献步骤、开发前置条件与检查方式见[贡献指南](CONTRIBUTING.md)。提交 Issue 请使用仓库模板并附环境信息（平台与架构、Herdr / Codex CLI 版本、安装方式、配置路径情况）；安全问题请使用[私有漏洞报告](SECURITY.md)，不要在公开 Issue 中包含凭据或账号信息。术语定义（如“当前账号”）见 [GLOSSARY.md](GLOSSARY.md)。

## 开源许可证

本项目以 [MIT 许可证](LICENSE)开源，预编译发行包同样按 MIT 分发；第三方依赖的许可清单见[许可证与第三方声明](docs/license-notices.md)。
