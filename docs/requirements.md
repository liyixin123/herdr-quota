# 初步需求：Herdr TUI 全局订阅额度

## 已确认的用户需求

- 使用 Herdr TUI。
- 开发自己的订阅额度插件。
- 首期只支持 Codex，后续可扩展其他服务。
- 显示 5h 剩余额度和 1w 剩余额度。
- 全局显示。
- 开发语言暂定 Rust。
- 项目目录：`/Users/liyixin/repos/rust/herdr-quota`。

## 首期验收目标

1. 读取当前所选 Codex 订阅账号的真实额度，分别展示 5h 和 1w 剩余百分比。
2. workspace、tab、pane 切换后，额度仍可见；不要求当前 pane 正在运行 Codex。
3. 明确标注“剩余”，避免将已用比例当作剩余比例。
4. 初次读取失败、未登录或账号不提供对应窗口时，显示未知或不可用，不伪造为 0% 或 100%。
5. 刷新失败时区分旧缓存与新数据，避免把过期值显示为实时额度。
6. 不把认证凭据写进项目文件或日志。

第 3—6 项为初始实现建议，后续可调整。自动刷新间隔、重置时间展示、颜色阈值和多账号支持尚未确定。

## 待验证的两个核心问题

### Herdr TUI 的全局展示入口

目标是常驻且跨 workspace 可见。先核对本机版本是否支持插件可写的全局状态栏、侧栏区域或显示 token。

官方 CLI 文档提供 pane/workspace 的 metadata token；这不能直接证明存在全局 token 或全局常驻区域。插件 pane 或 popup 也不能直接视为满足全局显示要求。若现有插件 API 无法满足，应先记录限制和备选方案，再决定是否需要修改 Herdr。

### Codex 订阅额度的数据来源

优先验证本机 Codex CLI/app-server 是否提供账号额度读取接口及更新通知，明确鉴权要求、字段含义、窗口时长和重置时间。

- 不用 token 消耗量估算订阅剩余额度。
- 不把 API 余额当作订阅额度。
- 不硬编码 primary/secondary 与 5h/1w 的对应关系，应根据接口返回的窗口信息验证。
- 本阶段未读取认证文件，未调用账号额度接口，尚未验证可获取真实数据。

## 建议实施顺序

1. 验证 Codex 额度数据接口，保存脱敏后的字段示例。
2. 验证 Herdr TUI 全局展示入口，并确认切换工作区后的行为。
3. 使用 Rust 确定依赖，编写最小的“读取 → 归一化 → 展示”实现。
4. 加入刷新、缓存、错误状态和进程退出处理。
5. 本地 link 插件，验证真实额度和全局可见性。

## 本机基线

2026-11-17 读取 CLI 输出：

- Herdr：`0.9.5`
- Codex CLI：`0.169.0`
- `codex app-server --help` 可用；帮助中标注为 experimental。

## 参考资料

- [Herdr TUI 插件](https://herdr.dev/tui/docs/plugins/)
- [Herdr TUI CLI](https://herdr.dev/tui/docs/cli-reference/)
- [Herdr TUI Socket API](https://herdr.dev/tui/docs/socket-api/)
- [Herdr TUI 配置](https://herdr.dev/tui/docs/config-reference/)

在线文档可能领先于本机版本，最终以本机能力和实际验证为准。
