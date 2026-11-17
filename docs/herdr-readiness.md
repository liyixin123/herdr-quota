# Herdr 全局展示能力准备记录

核对日期：2026-11-17。本机 Herdr：`0.9.5`。目标见 [初步需求](requirements.md)：切换 workspace、tab、pane 后仍能看到 Codex 的 5h / 1w **剩余**额度，当前 pane 无须运行 Codex。

## 结论

**采用 `ui.tab_bar_right` 的 `command` 状态项。** 官方文档提供了 tab 行右侧定时执行外部命令的入口；本机 0.9.5 的隔离会话已通过静态文本的 workspace、tab、pane 切换验收。宽屏下字符条与倒计时不依赖 Codex pane；窄屏隐藏并在恢复宽度后重新出现。测试详情见下文。

**不能将现有插件 pane 或 metadata 直接认定为满足全局常驻。** 插件 v1 不提供原生非终端 UI；pane / workspace metadata 分别属于对应资源；插件全局安装也不意味着它的显示区域全局常驻。[插件能力](https://herdr.dev/tui/docs/plugins/#overview)、[安装作用域](https://herdr.dev/tui/docs/plugins/#install-and-link)、[metadata 作用域](https://herdr.dev/tui/docs/socket-api/#agent-state-reporting)

文档明确说明状态区会在窄 tab 行上整体让位给 tabs 和控件；底部 tab 行在某些交互模式下会被临时替换。因此，若“全局”要求包括所有尺寸、模式下从不消失，当前证据不能证明已满足；若目标是桌面正常布局中切换 workspace / tab / pane 后继续显示，则现有状态项值得先做最小验收，无须现在决定修改 Herdr。[UI 与状态区](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar)

### 进度条与重置倒计时

用户进一步确认全局常驻，偏好进度条，并需要重置倒计时。`command` 只使用最后一行输出，因此两窗口必须排在同一行，不能用两行分别展示 5h / 1w。它剥离 ESC 控制序列，不支持通过 ANSI 颜色或光标移动绘制进度条；可输出普通字符条和倒计时，例如 `5h 剩余 [####--] 67% 02:15 | 1w 剩余 [##----] 33% 3d04h`。这只是格式示意，百分比与时间均为假数据；Unicode 字符宽度、中文占宽和具体排版仍待本机验证。[输出规则](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar)

官方文档没有给出 command 文本的固定字符上限，也没有承诺条目内部自动缩短、两项自动分行或窄屏优先保留某窗口；明确的行为是整个状态区让位。不能把“未写上限”解释为无限可见。程序可自行生成更短文本，但文档没有确认状态命令会收到可用状态区宽度，因此暂不承诺自动按宽度退化。[宽度行为](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar)

用户已选择保留两个短字符条，并接受窄屏时隐藏状态区。首期仅支持本机 Herdr server，读取同机 Codex 当前账号。该展示范围已通过静态文本切换验收；真实额度程序接入后的端到端行为仍须验证。

倒计时文本只随 command 刷新而更新。用户已确认按分钟更新倒计时、每 60 秒读取额度；重置时间缺失时显示“重置未知”，归零触发刷新且不自动补满额度。显示调度与读取可分开，但实际分钟刷新行为仍需本机验收。[调度间隔](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar)

## 官方文档确认的能力

| 入口 | 已确认能力 | 对本项目的意义与边界 |
| --- | --- | --- |
| `ui.tab_bar_right` | 支持 `zoom`、`hostname`、`datetime`、`text`、`command`；默认空数组。 | 是 Herdr 自带配置入口，不是插件 manifest 新增的 UI。可执行项目 Rust 程序输出一行额度文本。[配置参考](https://herdr.dev/tui/docs/config-reference/#ui-and-sidebar) |
| command 状态项 | 立即执行，之后按 `interval_seconds` 执行；同一命令不重叠。取成功输出的最后一行，剥离 ESC 控制序列；失败、空输出、超时会清空该项。 | 程序应自行将未知、不可用、旧缓存等状态输出为成功的一行文本；不能依赖 Herdr 在失败后保留上次文本，也不能依靠 ANSI 着色。[详细行为](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar) |
| command 执行环境 | 在 Herdr server 上运行；收到 active workspace / tab / pane 等上下文。Linux/macOS 使用 `/bin/sh -lc`。 | 额度读取逻辑应独立于 active pane。远程连接显示服务器侧数据，不能默认表示本地 Codex 账号。[详细行为](https://herdr.dev/tui/docs/configuration/#ui-and-sidebar) |
| pane / workspace metadata | 可更新命名 token，支持 TTL、序号；分别用于 Agent / Space sidebar rows。 | 可作备选，但属于每个资源；没有据此证明的 session 全局 token。向所有 workspace 复制额度还需处理新建资源和过期，不等于固定全局区域。[CLI metadata](https://herdr.dev/tui/docs/cli-reference/#panes)、[Socket metadata](https://herdr.dev/tui/docs/socket-api/#agent-state-reporting) |
| 插件 v1 | Rust 二进制可作为命令；CLI 就是插件 API；manifest 声明 action、event、pane 等。原生非终端插件 UI 不在 v1 范围。 | 可以包装额度程序，但不能仅凭 manifest 声明常驻原生状态栏。[插件总览](https://herdr.dev/tui/docs/plugins/#overview) |
| 插件 pane | 普通布局 pane 随其 tab / workspace 存在；popup 是 session 模态资源并接收全部终端输入。 | 普通 pane 的布局位置和 popup 的模态行为都不能直接满足后台常驻额度显示。[Panes](https://herdr.dev/tui/docs/plugins/#panes) |

外层终端标题也有 `client.window_title.set` / `clear`，但显示位置取决于宿主终端，不等同于 Herdr 内部常驻额度区，暂不作为首选。[Socket 方法](https://herdr.dev/tui/docs/socket-api/#raw-methods)、[外层标题](https://herdr.dev/tui/docs/configuration/#outer-terminal-window-title)

## 本机只读验证

以下结果来自前期本机只读 CLI 输出，并非运行中 UI 验收。该阶段未读取认证文件、修改配置、link 插件或创建运行资源；后续获得授权的隔离 UI 验收单列在下一节。

| 命令 | 观察结果 |
| --- | --- |
| `herdr --version` | `herdr 0.9.5` |
| `herdr --help`、`herdr plugin --help` | 插件子命令实际存在；顶层帮助的命令列表没有列出 plugin，不能仅靠顶层列表判定其缺失。 |
| `herdr --default-config` | 注释列出 `tab_bar_right = []`、separator、五种类型；说明 command 在 server 上解析。`hide_tab_bar_when_single_tab` 的注释默认值为 false。这里只打印默认配置，没有覆盖用户配置。 |
| `herdr workspace --help`、`herdr workspace report-metadata --help`、`herdr pane --help` | 本机确有 workspace / pane metadata 命令与 `--token`、`--ttl-ms` 等选项。 |
| `herdr api schema`、`herdr api schema --json` | 内置 schema：protocol `22`、schema version `1`。请求包含 workspace / pane metadata、plugin pane、window title；没有发现命名为 global/session metadata 或状态区写入的请求。此检索不能证明所有未公开能力不存在。 |
| `herdr plugin pane open --help` | CLI placement 只列 `overlay`、`split`、`tab`、`zoomed`，未列 popup；但内置 raw schema 的 `PluginPaneOpenParams` 已包含 width / height，且存在 `popup.close`。官方在线文档与本机 CLI 包装存在差异，popup 支持程度仍需单独验证。 |

本机 schema 另有 `client_shell.surface.set`，描述是控制请求客户端 shell 是否接收和控制 pane presentation，仅有 `active: boolean` 参数；没有据此将它解释为可写的插件展示区域。

## 本机隔离 UI 验收

2026-11-17 经用户授权，在独立命名会话 `herdr-quota-test-direct` 中验证。先读取了 `herdr --skill`；当前 agent 不在 Herdr pane 内，本次仅按用户明确要求控制自建测试会话，没有控制当前 focused session。配置与状态通过 `HERDR_CONFIG_PATH`、`XDG_CONFIG_HOME`、`XDG_STATE_HOME`、`XDG_DATA_HOME`、`XDG_RUNTIME_DIR` 全部指向临时目录 `/tmp/hq-l7oq07b5`。没有覆盖用户配置或安装插件。

使用终端工具原生 PTY 启动 Herdr 0.9.5，测试 shell 为 `/bin/sh`，没有运行 Codex。静态命令每秒输出 `QUOTA_TEST 5h [####--] 67% 2h15 | 1w [##----] 33% 3d4h`；数值仅为展示假数据。通过明确的测试 session、API 返回的资源 ID 操作，并捕获包含光标位置的 ANSI 重绘，检查状态文本位于顶部 tab 行，而非 pane 输出。

| 场景 | 实际结果 |
| --- | --- |
| 180 列、40 行，初始 workspace 单 tab | 顶部显示两窗口完整文本。 |
| 创建并聚焦第二 workspace `w2`，再新建并聚焦 tab `w2:t2` | 顶部状态保持；181 列强制全屏重绘仍含完整状态文本。 |
| split 并聚焦新 pane `w2:p3`，再切回 `w2:p2` | 两个没有 Codex 的 shell pane 间切换后，状态仍显示。 |
| 缩到 70 列、40 行 | 完整重绘中不再出现 `QUOTA_TEST`；tabs 与 pane 继续显示，状态区整体隐藏。70 列只是本次观测点，不是通用隐藏阈值。 |
| 恢复 180 列，再切回初始 workspace `w1` | 状态重新出现；切回 workspace 后的 181 列全屏重绘仍含完整文本。 |

ANSI 捕获保存在临时证据文件 `/tmp/hq-l7oq07b5/ui-evidence.json`，包含宽屏 tab、pane、70 列、恢复宽屏及切回 workspace 的输出。它是文本验收证据，不是像素截图，也没有真实账号信息。完成后 `session stop` 与 `session delete` 均返回成功，仅测试会话及其 panes 被清理，测试客户端随服务器关闭退出。

Python 自建 PTY 的先行尝试没有建立 API socket；最初还遇到 macOS Unix socket 路径过长，改短临时路径后解决。后续采用获准的原生 PTY 完成上述验收，没有将先行失败视为状态区能力失败。

### 实际 Rust 输出与 Unicode 补验

同日另建隔离会话 `herdr-quota-real-test`，配置与会话状态位于 `/tmp/hq-real-check`，使用实际发布二进制 `/Users/liyixin/repos/rust/herdr-quota/target/release/herdr-quota`。状态 command 的参数为 `status --cache-dir /tmp/hq-real-check/cache --codex /opt/homebrew/bin/codex`，`interval_seconds = 5`、`timeout_seconds = 2`。在 180/181 列 × 40 行下切换到新 workspace，确认实际程序的中文未知态以及 Unicode 条形额度态均位于顶部 tab 行。

测试缓存的两次初始 refresh 返回 `account_unknown`；与此独立，主实现验证在正常环境用 release 二进制和相同 Codex 路径成功取得额度。隔离测试使用了临时 XDG 配置环境，可能影响 Codex 登录存储解析；本次未诊断其原因，不能据此认定主程序读取不稳定，也没有将未知态伪装成有效额度。

为避免重复请求，额度态补验采用主实现成功读取的规范化缓存副本，**属于缓存回放，不是本隔离会话的实时获取**。副本仅将 `attempted_at` / `fetched_at` 调整为当前时间，额度与重置时间保持原值，没有输出身份摘要或认证信息。实际顶部行捕获为：

```text
Codex 剩余 · 5h [█████░] 96% ↻4h55m · 1w [█░░░░░] 30% ↻3h22m
```

完整文本在同一 tab 行，无可见的换行、重叠或截断；ANSI 重绘的行定位均为第 1 行，pane 的 `sh-3.2$` 位于第 2 行。该检查基于终端输出与光标定位，未做字体像素截图验收。证据：`/tmp/hq-real-check/ui-replay-evidence.json`。随时间推移还观察到分钟倒计时变化和“过期”标记；没有由此声称网络刷新成功。测试会话的 stop / delete 均成功，未覆盖用户配置或操作当前会话。

## 待验证与接入建议

1. 将 Rust 程序的 `status` 一行输出接入同样的 command 配置；建议 Herdr 每 5 秒调用一次、超时 2 秒。`status` 应立即输出缓存，网络额度读取由独立刷新处理、维持用户确认的 60 秒周期，倒计时保持分钟精度。这样启动后能较快显示首次刷新结果，不把慢网络阻塞放进状态栏。
2. 用真实程序补验初次无缓存、账号不可用、刷新失败旧缓存、重置时间缺失及倒计时归零；本次静态 UI 测试没有验证这些数据行为。
3. 尚未验收 zoom、64 列及以下移动布局、所有交互模式、重新连接和多客户端；也未实测 ANSI 剥离、失败清空、超时等异常分支，这些目前仍以官方文档为依据。
4. **首期不需要 `herdr-plugin.toml`。** 状态区直接执行 Rust 二进制即可，manifest 不会自动添加这个配置区域。若后续需要 Herdr 插件分发、action 或详情 pane，再增加插件包装；不要为了全局文本显示额外创建 pane。

推荐的接入配置（字段与本机静态验收一致；实际二进制路径需替换，尚未写入用户配置）：

```toml
[ui]
hide_tab_bar_when_single_tab = false
tab_bar_position = "top"
tab_bar_right = [
  { type = "command", command = "/absolute/path/herdr-quota status", interval_seconds = 5, timeout_seconds = 2 },
]
```

本文件只判断 Herdr 展示承载能力；真实额度来源和程序验证见项目其余实现文档。含空格的二进制路径需按 shell 规则引用；在 macOS 上该 command 由 `/bin/sh -lc` 解释。
