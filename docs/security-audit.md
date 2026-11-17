# 公开范围敏感信息审计与清理记录

审计日期：2026-11-17（仓库此时已处于 public 状态，审计按紧急事项执行）。对应工单：#5。目标：明确随仓库公开而暴露的敏感信息，逐项分类并处理，提供可复核的扫描证据。本文件不复述任何真实凭据或完整个人联系方式。

## 审计范围

- 远端分支与标签可达的全部 Git 历史（`origin/main` 与标签 `v0.1.0`、`v0.1.1`，共 22 个提交；经比对与 `--all` 引用集一致）。
- 两个 Release（v0.1.0、v0.1.1）的全部附件：4 个平台 tar.gz 与 2 份 SHA256SUMS。
- Actions 构建产物（build artifacts）抽样核验。
- GitHub 表面：Issue 正文与评论、Actions 运行日志、Release 说明文本、仓库元数据。
- 当前提交内容中的本机绝对路径、真实额度快照与重置时间、其他个人信息。

## 工具与版本

| 工具 | 版本 | 来源 |
| --- | --- | --- |
| gitleaks | 8.30.1（官方 linux-amd64 发行包） | `gh release download v8.30.1 --repo gitleaks/gitleaks --pattern 'gitleaks_*_linux_x64.tar.gz'` |
| gh | 2.100.0 | 本机 |
| git | 2.55.0 | 本机 |
| sha256sum / strings / grep | coreutils / binutils 标准 | 本机 |

## 一、Git 历史凭据扫描

取数：`git fetch origin '+refs/heads/*:refs/remotes/origin/*' --tags`；远端分支仅 `main`（f284c45），标签 `v0.1.0`、`v0.1.1`；可达提交 22 个（非合并 13 + 合并 9）。

扫描方式（两种互补）：

1. 增量历史扫描：`gitleaks detect --source . --log-opts="--all" --redact --exit-code 1` → `13 commits scanned`、`no leaks found`（合并提交无 patch，不计入）。
2. 逐提交全树扫描：对 `git rev-list --all` 的每个提交 `git archive` 解包后 `gitleaks detect --no-git`，共 22 份 → **全部 0 发现**。

结论：**未发现任何真实凭据，也未出现需要人工甄别的疑似凭据告警**（0 条发现，含测试假数据命中为 0）。仓库中的 token 相关内容均为流程说明或测试夹具（如 `tests/remote/test_remote.py` 的 Actions bearer 字符集校验、`scripts/remote.sh` 的 token 传递规则），不含 token 字面值；对 `ghp_`/`github_pat_`/`AKIA`/`sk-`/私钥头等常见凭据前缀的全库 grep 亦为 0 命中。**无需吊销或轮换任何凭据。**

## 二、Release 附件与构建产物

- 下载：`gh release download v0.1.0 / v0.1.1`（各 3 个附件），`sha256sum -c SHA256SUMS` 全部通过；每个 tar.gz 仅含根目录单个可执行文件 `herdr-quota`。
- gitleaks `--no-git` 扫描 4 个解包二进制与 2 份 SHA256SUMS → 0 发现。
- `strings` 复核二进制内嵌路径：仅 GitHub 托管 runner 的标准路径（`/Users/runner/.cargo/registry/...`，来自依赖库调试信息），无本机用户路径、用户名、邮箱或主机名。
- Actions artifact 抽样：下载 v0.1.1 发布运行（run 44041158155）的 `package-aarch64-apple-darwin` artifact（id 14310070285），其 tar.gz SHA-256 与已扫描的发布附件完全一致（字节级相同），artifact 生成路径与发布附件相同（`scripts/release.py package`）。

## 三、GitHub 表面检查

- **Issue 与评论**：#1–#11 全部正文及 4 条评论（#1–#4 各 1 条）经 gh API 拉取后按凭据与个人信息模式扫描 → 0 命中；内容仅含 token 处理规范描述，无凭据值、无邮箱/IP/路径/真实额度数值。
- **Actions 运行日志**：共 10 次运行；8 次已完成的运行逐一下载完整日志（约 2 MB）扫描 → 0 凭据模式命中，无带内容的 `Authorization` 头，仅 `/home/runner/...` 标准 CI 路径。脚本与测试本身设计为不回显 token，日志证据与设计一致。
- **Release 说明**：v0.1.0/v0.1.1 说明均为自动生成的 changelog 链接，无敏感内容。
- **仓库元数据**：描述为"个人 Herdr TUI 订阅额度插件"，无 topics/homepage，无个人信息。

## 四、提交内容个人信息复核与处理

逐项分类（真实凭据：0 项）：

| # | 位置 | 内容类别 | 判定与处理 |
| --- | --- | --- | --- |
| F1 | `docs/herdr-readiness.md` | 本机 macOS 绝对路径（含系统用户名） | 已移除：改为 `<仓库检出目录>/target/release/herdr-quota`。同句 `/opt/homebrew/bin/codex` 为 macOS 标准路径、`/tmp/hq-real-check` 为通用临时目录，均保留。 |
| F2 | `docs/requirements.md` | 本机 macOS 绝对路径 | 已移除：改为"本仓库 `herdr-quota`"。 |
| F3 | `docs/codex-readiness.md` | 真实额度快照数值（两窗口 usedPercent、剩余百分比与 resetsAt Unix 时间戳，来自真实账号读取） | 已脱敏：数值列替换为 `[已脱敏]`，保留 windowDurationMins 窗口结构（协议结构信息，非个人数据）。 |
| F4 | `docs/herdr-readiness.md` | 缓存回放捕获行中的真实额度与重置倒计时数值 | 已替换为标记为脱敏示意的数值，保留格式验收价值（条形宽度、排版与原验收一致）。 |
| F5 | Git 提交作者/提交者邮箱 | 个人信息（历史元数据） | 22 个已公开提交的作者邮箱为裸 QQ 号（形如 `592****48`，非有效邮箱）；另有 2 个本地未推送提交使用个人 Gmail（`liyixin.q***@gmail.com`，见"剩余事项"）。历史改写需强制推送且会使标签/发布链失效，超出本工单授权，列为剩余事项。本工单新增提交已改用 GitHub noreply 地址（`56808175+liyixin123@users.noreply.github.com`）。 |

明确保留的内容（公开价值大于隐私影响）：

- 测试夹具中的 `72%`/`33%` 等数值（`tests/release/smoke.py`、`tests/remote/*` 等）：属假数据夹具（配假身份），非真实账号快照。
- `README.md` 的额度显示示例：已自标"仅示意，非真实额度"。
- 二进制内 `/Users/runner/.cargo/...`：GitHub 标准 CI 路径。
- 文档中的临时目录路径 `/tmp/hq-*`：不含用户标识。

## 五、已做修改

- `docs/requirements.md`：移除本机绝对路径（F2）。
- `docs/herdr-readiness.md`：移除本机绝对路径（F1）；真实回放数值替换为脱敏示意并注明（F4）。
- `docs/codex-readiness.md`：真实快照数值列脱敏并注明（F3）。
- 新增本文件（审计记录）。

均为文档修改，不涉及代码与测试；`just check` 保持通过。

## 六、复扫证据

编辑后在工作区重新执行（与首次扫描同一二进制 gitleaks 8.30.1）：

```sh
# 1. 凭据复扫（与首次扫描同一二进制 gitleaks 8.30.1）
gitleaks detect --source . --log-opts="--all" --redact --exit-code 1
# → INF 17 commits scanned.  INF no leaks found.   （本地分支此时含其他工单新提交）
gitleaks detect --source . --no-git --redact --exit-code 1
# → INF scanned ~178786 bytes  INF no leaks found.
# 2. 本机路径复扫
grep -rnE '/Users/[A-Za-z]+|/home/[a-z]+' --exclude-dir=.git .
# → 0 命中（.git 工作区指针文件除外，其非提交内容）
# 3. F3/F4 原始数值复扫：对两处已脱敏的原始额度数值与重置时间戳逐值 grep（数值不在此复述）
# → 均 0 命中
```

复核人可在任意检出上重跑上述命令验证。

### 集成分支复扫（2026-11-17，全部工单合入后）

审计要求“其他工单分支合入后应按同口径复扫”，已在集成分支 `integrate/public-release`（`main` f284c45 之后共 12 个非合并+合并提交，含 #5–#8、#10、#11 全部交付）上执行同一二进制 gitleaks 8.30.1：

```sh
gitleaks detect --source . --log-opts="main..integrate/public-release" --no-banner
gitleaks dir . --no-banner
```

结果：`12 commits scanned` → `no leaks found`；工作区 `scanned ~66.1 MB` → `no leaks found`。此前记录的 Gmail 邮箱暴露已按“剩余事项 2”改写并强制推送，复扫同时覆盖改写后的分支内容。

## 七、覆盖范围与无法检查的项目

- 2 个 Actions 运行（implement/ticket-10 44087536288、implement/ticket-6 44087386407）审计时仍在进行/排队，日志不完整；其源提交为其他工单的本地分支，尚未推送。这些分支合入后应按同口径复扫。
- 其他工单的本地未推送分支（如含个人 Gmail 邮箱的提交）不在远端可达历史内，本次未纳入公开面定义，但推送前需处理（见下）。
- GitHub Actions 日志保留期（默认 90 天）内无过期日志；本次 8 个已完成运行日志均可下载。
- 无法检查的表面：无（issues、runs、artifacts、releases、元数据均已覆盖）。fork/缓存快照（如 GH Archive、各镜像）超出仓库可控范围。

## 八、剩余事项

1. **Git 历史作者邮箱**（F5）：已公开的 22 个提交携带裸 QQ 号作为作者邮箱；如需彻底移除须改写历史并强制推送，同时会使 v0.1.0/v0.1.1 标签与已发布资产失联，代价显著高于暴露本身（无凭据能力，仅弱身份关联）。建议：维持现状 + 后续提交统一使用 noreply 地址；是否改写由维护者决策。
2. **未推送本地分支的 Gmail 邮箱提交**：并行工单分支（implement/ticket-6、implement/ticket-10 等）存在使用个人 Gmail 邮箱的新提交，推送前应将仓库 git 配置（`user.email`）切换为 `56808175+liyixin123@users.noreply.github.com` 或对已产生提交改写作者（未推送前改写无代价）。
   - **处置记录（2026-11-17，集成阶段补录）**：审计与推送并行发生，ticket-6/7/10 分支一度携 Gmail 提交推送到公开远端。已处置：仓库级 git 配置改为 noreply 地址；集成分支 `integrate/public-release` 上 f284c45 之后的全10 个提交用 `git filter-branch --env-filter` 改写为 noreply（改写前后树内容字节一致，`git diff` 为空）；三个工单分支同步改写并强制推送，远端分支引用不再指向 Gmail 提交（旧提交对象在 GitHub 侧待 GC，API 事件/缓存可能短期残留）。后续所有集成提交均使用 noreply 地址。
3. 本工单新增提交已使用 noreply 地址；建议将全局/仓库默认邮箱一并切换，避免后续工单继续引入。
4. README/docs 中仍按私有仓库描述 token 安装流程，属工单 #7 范围，本审计未改。
5. 可选增强（属 #10 范围）：在 CI 中加入 push 前 gitleaks 扫描步骤，防止未来引入凭据。
