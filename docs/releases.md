# 预编译发行包

版本标签使用 `v<VERSION>`，例如 `v0.1.0`，必须与 `Cargo.toml` 的包版本一致。GitHub Actions 在 tag push 时构建、测试并发布；PR 和手动 `workflow_dispatch` 只验证并上传构建产物，不创建 Release。手动运行时选择目标分支。发布需要仓库维护者推送标签，不能用分支名充当版本。

稳定下载契约：

| 平台 | 文件名 | 运行时范围 |
| --- | --- | --- |
| macOS ARM64 | `herdr-quota-v<VERSION>-aarch64-apple-darwin.tar.gz` | macOS 15.0 或更新版本；在 macOS 15 ARM64 runner 原生验证 |
| Linux x86_64 | `herdr-quota-v<VERSION>-x86_64-unknown-linux-musl.tar.gz` | 静态 musl，无 glibc 或动态加载器依赖；支持范围以 Ubuntu 22.04 及更新版本的 x86_64 Linux 为基线，在 Ubuntu 22.04 runner 验证 |

每个 gzip tar 包根目录只有一个可执行文件 `herdr-quota`，解压后可以直接运行，无需 Rust。`SHA256SUMS` 使用标准 `sha256sum` 格式，列出两个包的 SHA-256 摘要及完整文件名。发行包按 MIT 分发，其中静态链接的第三方组件遵循各自的许可条款，见[许可证与第三方声明](license-notices.md)。

资产浏览器地址为：

```text
https://github.com/liyixin123/herdr-quota/releases/download/v<VERSION>/herdr-quota-v<VERSION>-aarch64-apple-darwin.tar.gz
https://github.com/liyixin123/herdr-quota/releases/download/v<VERSION>/herdr-quota-v<VERSION>-x86_64-unknown-linux-musl.tar.gz
https://github.com/liyixin123/herdr-quota/releases/download/v<VERSION>/SHA256SUMS
```

仓库为公开仓库，以上地址可匿名下载。维护者的发布验证仍使用 `GH_TOKEN` 和 GitHub Release asset API 下载同名资产；私有镜像或 fork 的下载需要对应仓库的读取权限。

下载后先校验再解压执行。例如 Linux 使用 `sha256sum --check SHA256SUMS --ignore-missing`；macOS 使用 `shasum -a 256` 对照清单中的对应文件摘要。校验信息用于发现下载损坏，下载资产和清单必须来自可信的同一个 Release。

本地验证（不发布）：

```sh
cargo build --release --locked
python3 -m unittest discover -s tests/release
```

在 Linux 上设置 `RELEASE_TEST_BINARY` 为真正的 `x86_64-unknown-linux-musl` 可执行文件。测试将打包、解压并执行版本查询，通过隔离的 Codex app-server fixture 检查额度读取和状态输出，并从本机 HTTP 服务下载资产验证校验值及拒绝损坏的包。可执行文件运行时的 `PATH` 不包含 Rust、Codex 或其他开发工具，只包含 fixture；不会访问真实账号。

CI 对两个目标分别运行 Rust 测试、编译、解压后的程序和下载校验测试。Linux 额外拒绝带 `INTERP` 动态加载器的包。所有平台验证完成才创建 Release，然后重新通过 GitHub API 下载两个平台的包和校验清单，检查浏览器 URL、必需资产与 SHA-256。手动检查现有 Release：

```sh
GH_TOKEN=... python3 scripts/verify-published.py --tag v0.1.0 --output /tmp/herdr-release-check
```

发布验证失败会使 workflow 失败，已经创建的 Release 保留供维护者诊断；不要覆盖同版本资产，修复后递增版本重新发布。仓库尚无实际发布时，只能完成本地及 CI 构建验证，无法声称线上下载已验证。

Runner 架构以 [GitHub 官方 runner 参考](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) 为依据，并由运行时 `uname` 检查，避免 runner 标签变更后误测架构。
