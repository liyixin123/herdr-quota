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

## 远程发行包入口

普通用户使用 README 的远程命令。`scripts/remote.sh install [原生参数]` 和 `scripts/remote.sh uninstall [原生参数]` 都先下载临时管理程序；不依赖已安装程序存在或支持当前管理接口。shell 不合并配置、不删除安装文件，统一调用经过验证的临时原生程序。原生程序的失败退出码会传递到调用者；成功提示以真实配置保存和 Herdr 重载结果为准。

支持 Darwin ARM64 和 Linux x86_64，发行包名称遵循 `herdr-quota-v<VERSION>-<target>.tar.gz`，使用同一 Release 的 `SHA256SUMS`。公开仓库的匿名路径是默认入口，从 latest 校验清单推导版本并固定后续下载，不需要 token 或 `jq`；私有镜像或 fork 用 `GH_TOKEN`（优先）或 `GITHUB_TOKEN` 访问 GitHub API，要求 `jq`。token 使用私有临时 curl 配置传入，不进入命令参数或日志；API 不自动跟随重定向，受信任的 GitHub 资产重定向使用独立的无凭据请求。临时目录退出时清理。入口 bootstrap 固定在 `v0.1.1`（README 主入口从 raw.githubusercontent.com 匿名获取），而程序默认选择最新 Release；`HERDR_QUOTA_TAG=v<VERSION>` 可固定发行版用于验收。

验证 SHA256 后，先要求 tar 只有根目录下单个普通文件 `herdr-quota`，再解包，核对 `--version` 与 Release tag 一致，最后执行 install/uninstall。下载、校验、归档或版本验证失败都发生在调用原生安装前，不会覆盖现有程序。SHA256 用于检测下载一致性，不是独立的发行者签名。发行源是本仓库的 GitHub Release。

远程回归需要开发环境的 Python 3.11+、Rust 和 jq（认证路径测试用），以及访问真实公开端点的网络（匿名回归），不是用户安装前置条件。`tests/remote/build_previous.py` 构建同一原生管理实现的 `0.0.0` 版本夹具，`tests/remote/test_remote.py` 把真实 release 可执行文件打包，并在隔离 HOME/XDG、Codex/Herdr 和传输边界夹具下运行远程命令，验证已有配置、安装、重复、旧版更新、下载/校验失败保留、重试与卸载，也实际执行生成配置中的 shell command 读取额度。`tests/remote/test_anonymous.py` 在清除 `GH_TOKEN`/`GITHUB_TOKEN` 的隔离环境下访问真实公开端点：raw 入口脚本、latest `SHA256SUMS` 与发行包，独立校验摘要后覆盖首次安装、重复安装、旧版更新、下载与校验失败保留及重试、手动删除后卸载和重复卸载。

CI 的两个原生 runner 分别运行 macOS ARM64 和 Linux x86_64 musl 包的完整回归；shell 平台模拟不能代替原生执行。tag 发布之后，`published-lifecycle` 在两个平台运行 `tests/remote/published.py`，从固定 tag 的真实 Contents API 获取入口，再通过真实 Release API 下载并安装、重复安装、删除已安装程序后卸载及重复卸载。发布前只能对最新已发布 Release 完成实际下载验收，新版本本身要等 tag 发布后验收；本地回归的 Herdr 重载使用夹具，不代表已验证真实 Herdr UI。发布 job 需要 Contents write，构建和发布后验收仅需 Contents read。
