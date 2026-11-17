# CI 与仓库保护

对应 issue #10：主分支改动与外部贡献持续接受自动检查，维护者能发现依赖风险并应用合入保护。本文记录工作流、检查名称、保护与扫描状态、权限审计、验证证据及平台限制待办。文中 API 结果均取自 2026-11-17 实测。

## 工作流

### ci.yml（Continuous checks，本票新增）

- 触发：`push` 到 `main`、所有 `pull_request`、`workflow_dispatch`。
- 顶层权限：`permissions: contents: read`，无 job 级提权。
- 三个 job 完整镜像 `just check`，均运行在 `ubuntu-22.04`，工具链固定 `dtolnay/rust-toolchain@stable` + `toolchain: '1.89.0'`（与 release.yml、`Cargo.toml` 的 `rust-version = "1.89"` 一致）：

| job | 命令 | 说明 |
| --- | --- | --- |
| `fmt` | `cargo fmt --all -- --check` | 显式安装 `rustfmt` 组件 |
| `test` | `cargo test --locked` | 先装 `just 1.46.0`（`tests/install.rs` 会执行 `just install`/`just uninstall` 集成测试） |
| `clippy` | `cargo clippy --locked --all-targets -- -D warnings` | 显式安装 `clippy` 组件 |

检查名称即 job id：`fmt`、`test`、`clippy`（未设置 `name:` 时 check-run 名称等于 job id）。

### release.yml（Release packages，已有）

触发 `push` 到 `implement/**` 与 `v*` 标签、所有 `pull_request`、`workflow_dispatch`，双平台构建矩阵 + 标签发布 + 发布后生命周期验收，详见 `docs/releases.md`。其对 PR 的 check-run 名称为 `build (macos-15, aarch64-apple-darwin)` 与 `build (ubuntu-22.04, x86_64-unknown-linux-musl)`（由矩阵键生成，下文实测确认）。

## main 分支保护

### 现状（2026-11-17 实测）

- `GET /repos/liyixin123/herdr-quota/rulesets` → `[]`（0 个规则集）。
- `GET /repos/liyixin123/herdr-quota/rules/branches/main` → `[]`。
- `GET /repos/liyixin123/herdr-quota/branches/main/protection` → 404 Branch not protected。
- 即 main 当前无任何保护：可被强推、删除，PR 合并无强制检查。

### 目标设计（规则集 "main branch protection"）

对 `refs/heads/main`、`enforcement: active`：

1. `deletion`：禁止删除 main。
2. `non_fast_forward`：禁止强推（force push）main。
3. `required_status_checks`（非严格策略），必需检查全部为**实测存在**的名称：
   - `fmt`、`test`、`clippy`（ci.yml job id；命令已在本地 Rust 1.89.0 上逐条验证通过，见验证证据）
   - `build (macos-15, aarch64-apple-darwin)`、`build (ubuntu-22.04, x86_64-unknown-linux-musl)`（release.yml，名称取自分支实际 check-run）

选择说明：

- **未启用 `pull_request`（要求 PR 才能合入）规则**：本仓库发版机制（#8）依赖维护者直接推送合并到 main，且历史上无 PR 流程；启用该规则会阻塞直接推送。改用"必需状态检查约束 PR + 禁删/禁强推约束所有推送"的组合。
- **必需状态检查只约束 PR 合并**；直接推送到 main 不被其拦截（检查在推送后运行并可见）。因此直接推送仍能发版，失败的检查会立即在 main 最新提交上显红。
- **未配置任何 bypass**：规则（含禁删/禁强推）对包括管理员在内的所有人生效。
- `existing-release-lifecycle`/`publish` 等检查只在标签或 `implement/**` 推送时出现，PR 上永不报告，故不列入必需（否则 PR 永久卡在等待检查）。

### 本次未能在线启用（token 限制）

本机唯一凭据为细粒度 PAT（fine-grained PAT，仓库权限 `admin:true`），但以下写入端点全部返回 403 `Resource not accessible by personal access token`（该类端点需要经典 PAT 或网页端）：

| 调用 | 结果 |
| --- | --- |
| `POST /repos/liyixin123/herdr-quota/rulesets` | 403 |
| `PUT /repos/liyixin123/herdr-quota/branches/main/protection` | 403 |
| `PUT /repos/liyixin123/herdr-quota/vulnerability-alerts` | 403 |
| `PUT /repos/liyixin123/herdr-quota/automated-security-fixes` | 403 |
| `PATCH /repos/liyixin123/herdr-quota`（`security_and_analysis`） | 403 |

### 维护者执行手册（经典 PAT 或网页端，约 2 分钟）

方式一（经典 PAT，gh 已登录到 liyixin123 后直接粘贴）：

```sh
gh api -X POST repos/liyixin123/herdr-quota/rulesets --input - <<'EOF'
{
  "name": "main branch protection",
  "target": "branch",
  "enforcement": "active",
  "conditions": {"ref_name": {"include": ["refs/heads/main"], "exclude": []}},
  "rules": [
    {"type": "deletion"},
    {"type": "non_fast_forward"},
    {"type": "required_status_checks",
     "parameters": {
       "strict_required_status_checks_policy": false,
       "required_status_checks": [
         {"context": "fmt"},
         {"context": "test"},
         {"context": "clippy"},
         {"context": "build (macos-15, aarch64-apple-darwin)"},
         {"context": "build (ubuntu-22.04, x86_64-unknown-linux-musl)"}
       ]
     }}
  ]
}
EOF
gh api -X PUT repos/liyixin123/herdr-quota/vulnerability-alerts
gh api -X PUT repos/liyixin123/herdr-quota/automated-security-fixes
gh api -X PATCH repos/liyixin123/herdr-quota --input - <<'EOF'
{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}
EOF
```

方式二（网页端）：Settings → Rules → Rulesets → New ruleset → New branch ruleset：name `main branch protection`、Enforcement Active、Targets 选 `Target branches: refs/heads/main`，勾选 Require a pull request before merging 之外的三项：Restrict deletions、Block force pushes、Require status checks to pass（添加上表 5 个名称）；随后 Settings → Code security and analysis 开启 Dependabot security updates、Dependabot alerts、Secret scanning、Push protection。

验证：`gh api repos/liyixin123/herdr-quota/rulesets`（应出现 active 的规则集）与 `gh api repos/liyixin123/herdr-quota/rules/branches/main`（应列出 deletion、non_fast_forward、required_status_checks 及 5 个 context）。

## 依赖更新与安全扫描

- `.github/dependabot.yml`（本票新增）：`cargo` 与 `github-actions` 两个生态、目录 `/`、每周检查。版本更新（version updates）在文件合入 `main` 后生效。
- 安全能力现状（`GET /repos/liyixin123/herdr-quota --jq .security_and_analysis`，2026-11-17）：`dependabot_security_updates`、`secret_scanning`、`secret_scanning_push_protection` 等全部 `disabled`；`GET .../automated-security-fixes` → `{"enabled":false,"paused":false}`；`GET .../vulnerability-alerts` → 404（未启用）。启用受上节 403 限制，待维护者按手册执行。

## 工作流权限审计

全仓库仅有两个工作流：

- release.yml：顶层 `permissions: contents: read`；唯一的写权限在 `publish` job（`contents: write`，标签发布 Release 必需）；`published-lifecycle`、`existing-release-lifecycle` 显式 `contents: read`；`build` 继承顶层只读。**无任何更宽权限**（无 `write-all`、无其他 scope、无 job 提升）。
- ci.yml：顶层 `contents: read`，无 job 级权限，纯只读检查。
- 两者的第三方 action 均为 `actions/*` 官方与 `dtolnay/rust-toolchain`、`extractions/setup-just`，只获得所在 job 的 token 权限；仓库不使用任何 secret。

## 验证证据（2026-11-17）

1. 分支 `implement/ticket-10` 推送后 Release packages 运行 44087536288 全绿：`build (macos-15, aarch64-apple-darwin)` success、`build (ubuntu-22.04, x86_64-unknown-linux-musl)` success、`existing-release-lifecycle (macos-15/ubuntu-22.04)` success、`publish`/`published-lifecycle` 按条件跳过（非标签推送）。
2. 真实 check-run 名称取自 `GET /repos/liyixin123/herdr-quota/commits/6e4c2c998a7262a762425e770bad9dfcdea5b19e/check-runs`，名称与上表逐字一致。
3. ci.yml 三条命令在本地 Rust 1.89.0（`cargo +1.89.0`，`rustup toolchain install 1.89.0 --profile minimal --component rustfmt --component clippy`）上逐条通过：`fmt --check` OK、`test --locked` 28 个测试全过（含 just 集成测试）、`clippy -D warnings` OK——排除 1.95/1.89 工具链差异导致的 CI 失败。
4. 安全/保护端点 403 与现状 disabled 的原始响应见上文两表。
5. `gh workflow run ci.yml --ref implement/ticket-10` → 404 "workflow ci.yml not found on the default branch"（workflow_dispatch 需默认分支存在该文件），故 ci.yml 的在线首跑发生在本票合入 main 后。

## 平台限制与待办

1. **分支保护与安全扫描需维护者凭据启用**：细粒度 PAT 无法调用这些端点（403 表）。按手册执行后，用上面的验证命令记录生效证据。
2. **外部 fork PR 检查流程无法端到端验证**：需要第二个账号提 fork PR；本仓库无第二个账号可用。已知事实：ci.yml 与 release.yml 都监听全部 `pull_request`，对同仓分支与 fork PR 触发一致；两者顶层均为 `contents: read`、不使用 secret，fork PR 的只读令牌足以运行所有检查（release.yml 的构建矩阵、发布后验收不依赖写权限或 secret）。合入 main 后首个真实 PR（含 fork PR）可补全此验证。
3. **ci.yml 在线首跑**：合入 main 的那次推送会触发 `fmt`/`test`/`clippy` 首次在线运行；本地 1.89.0 已逐条验证同命令，风险仅剩 runner 环境差异（低）。
4. **dependabot 版本更新**在 `.github/dependabot.yml` 合入 main 后生效；安全更新（automated security fixes）待维护者按手册启用。
5. 直接推送到 main 不受必需状态检查约束（GitHub 平台行为：其只约束 PR 合并）；main 上检查失败靠推送后可见的红叉与后续治理兜底，发版流程（#8 的标签机制）不受影响。
