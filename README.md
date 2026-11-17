# Herdr Quota

个人 Herdr TUI 订阅额度插件，项目暂名 `herdr-quota`。

当前阶段：需求初始化，尚未实现或安装插件。

## 首期目标

在 Herdr TUI 中快速查看当前 Codex 订阅的剩余额度：

- 5 小时窗口（5h）剩余百分比。
- 一周窗口（1w）剩余百分比。
- 全局可见，切换 workspace、tab 或 pane 后仍能查看，无需进入某个 Codex 会话。

预期文案示例（仅示意，非真实额度）：

```text
Codex · 5h 78% · 1w 63%
```

详细需求和待验证事项见 [docs/requirements.md](docs/requirements.md)。

## 开发目录

```sh
cd /Users/liyixin/repos/rust/herdr-quota
```

开发语言暂定 Rust。依赖、清单最低版本和具体实现待接口验证后确定。本阶段没有运行命令。
