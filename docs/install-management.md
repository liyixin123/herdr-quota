# 本地安装与配置管理

开发者安装需要 `just`、Rust 1.89+、Herdr 和已用 ChatGPT 订阅账号登录的 Codex CLI。在项目目录执行 `just install`：编译 release 程序，安装到 `~/.local/bin/herdr-quota`，自动定位 Codex 的绝对路径，检查登录，备份并合并 Herdr 配置，然后执行 `herdr server reload-config`。

运行 `just check` 汇总格式检查、隔离回归测试和 Clippy。安装回归使用临时 HOME、配置、数据目录及可控 Codex / Herdr 程序，不读取真实认证信息，不修改真实配置。`just install` 回归用可控 cargo 代表编译边界；真实构建和制品验证另行执行。

安装失败时按照提示修复并重跑同一条命令。Herdr 没有启动或重载失败会返回失败，已安装程序和已保存配置保留用于重试。配置解析失败时原文件保持不变。已有配置的备份路径会打印出来；备份以 0600 创建。配置更新使用同目录临时文件和原子替换，保留原配置文件权限、注释、现有状态项和 tab 栏位置；只添加额度项并设置单 tab 显示 tab 栏。

二进制公开管理命令：

```sh
herdr-quota install
herdr-quota configure
```

`install` 安装当前二进制后接入 Herdr；`configure` 直接接入当前二进制。两者支持 `--config ABSOLUTE_PATH`、`--install-dir ABSOLUTE_PATH`、`--cache-dir ABSOLUTE_PATH`、`--codex EXECUTABLE` 和 `--herdr EXECUTABLE`。日常安装不需要这些参数。指定配置时应确保运行中的本机 Herdr 使用同一配置路径。

默认配置路径为 `HERDR_CONFIG_PATH`，其次为 `$XDG_CONFIG_HOME/herdr/config.toml`，最后为 `~/.config/herdr/config.toml`。安装路径固定为 `~/.local/bin/herdr-quota`；不要求该目录加入 PATH，因为配置引用程序绝对路径。

管理状态位于 `$XDG_DATA_HOME/herdr-quota/install.json`，默认 `~/.local/share/herdr-quota/install.json`，以 0600 原子保存，先于配置修改写入，以支持中断恢复。状态包含 binary、config、command、previous_hide（原 TOML 值文本或 null）、ui_existed。重复安装同一配置会保留最初的设置。额度 command 末尾的 shell 注释 ` # herdr-quota` 标记本程序管理的状态项，重试时替换原项避免重复；保留其他状态项。状态不保存账号或认证信息。

在项目目录执行 `just uninstall` 会先编译清理程序，再调用相同的原生命令 `herdr-quota uninstall`；即使已安装程序被手动删除，也能继续清理。卸载不需要 Codex 或登录信息，支持与安装相同的路径及 `--herdr` 参数。默认优先使用安装记录中的配置和程序路径；显式配置路径必须与记录一致。

卸载只移除带有精确后缀 ` # herdr-quota` 的额度状态项，保留其他状态项、注释、tab 栏位置及安装之后的配置修改。不会把整份安装备份覆盖回配置。单 tab 显示设置仍为安装写入的 `false` 时，才恢复记录中的原值或移除原本不存在的键；如果用户改为其他值则保留。原值也是 `false` 或用户后来再次设为 `false` 时无法区分来源，恢复行为以安装记录为准。没有安装记录时仍可清理标记项，但不猜测原先的显示设置。`configure` 直接使用的开发程序不会被删除；状态中的 `installed` 布尔值记录是否安装了程序副本。

配置变更前打印新的卸载备份路径，以 0600 创建；成功保存后先重载 Herdr，最后删除已安装程序和安装记录。解析、读写或重载失败会返回失败，报告剩余操作，并保留程序及安装记录供重试；已经保存的配置清理不回滚。重载失败后只有 `herdr status --json` 明确报告服务未运行时才完成删除，并提示配置清理将在下次启动生效。无法确认服务状态时保留恢复信息，修复后重跑同一卸载命令。卸载不会删除备份或额度缓存。重复卸载及已手动删除程序、配置或状态项均可安全重试。
