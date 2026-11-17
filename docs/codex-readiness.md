# Codex 订阅额度接口查证

查证日期：2026-11-17。基线：本机 `codex-cli 0.169.0`。

**结论：本机协议具备读取账号额度与接收更新的接口；经用户授权，真实 ChatGPT 账号的 5h/1w 读取已成功。账号切换与通知覆盖仍未验证。** 本次未直接读取认证文件，未调用登录、退出或切换账号接口，未修改用户配置；认证由 Codex app-server 自行处理。

## 运行时验证

2026-11-17，使用 `/private/tmp/herdr-quota-codex-probe.py` 启动独立 stdio app-server，完成 `initialize → initialized → account/read(refreshToken:false) → account/rateLimits/read(excludeResetCreditDetails:true)`。探针只输出账号类型、身份字段是否存在与窗口数字，丢弃 stderr，不输出 email、userId、accountId、bucket 标识或完整响应。设置总时限 30 秒，退出时终止并回收子进程。沙箱内首次运行在响应前 EOF，原因未记录；获授权的沙箱外运行成功。

观测到 `account.type = chatgpt`；`userId` 与额度 `accountId` 均存在，但未记录其值。返回一个 bucket，两个窗口如下；百分比和重置值均为数字。原始真实快照中的 `usedPercent`、剩余百分比与 `resetsAt` 数值已按敏感信息审计（见 [security-audit.md](security-audit.md)）脱敏，仅保留窗口结构：

| 返回位置 | windowDurationMins | usedPercent | 计算剩余 | resetsAt（Unix 秒） |
| --- | --- | --- | --- | --- |
| primary | 300 | [已脱敏] | [已脱敏] | [已脱敏] |
| secondary | 10080 | [已脱敏] | [已脱敏] | [已脱敏] |

这证明本次账号与本机版本能够取得目标窗口，不能保证所有账号都具有相同窗口或固定位置。未创建 thread 或模型 turn；未验证通知、切换账号、刷新失败及跨进程变化。

## 可复现的本机证据

执行：

```sh
codex --version
codex app-server generate-json-schema --out /private/tmp/herdr-quota-codex-schema-016900
```

生成成功，未使用 `--experimental`。以下引用 `S/...` 均指上述临时目录内的相对文件，JSON 路径用于定位；目录可按命令重新生成，不作为项目依赖。

| 已验证事实 | 本机 schema 证据 |
| --- | --- |
| 请求方法 `account/rateLimits/read` 存在；只要求 `id` 和 `method`，`params` 可省略或为 null | `S/ClientRequest.json` → `Account/rateLimits/readRequest` |
| 返回必需的 `rateLimits`，以及可选、可空的 `rateLimitsByLimitId` | `S/v2/GetAccountRateLimitsResponse.json` → `properties`、`required` |
| `rateLimits` 是兼容的单 bucket 视图；多 bucket map 的键是计量 `limit_id`，示例为 `codex` | 同上 → `properties.rateLimits.description`、`properties.rateLimitsByLimitId.description` |
| `primary`、`secondary` 均可缺失或为 null；窗口内只有 `usedPercent` 必需 | 同上 → `definitions.RateLimitSnapshot`、`definitions.RateLimitWindow` |
| `usedPercent` 为 int32；`windowDurationMins`、`resetsAt` 为可选、可空 int64；schema 未约束百分比上下界 | 同上 → `definitions.RateLimitWindow` |
| 更新方法是 `account/rateLimits/updated`，载荷为 `{rateLimits: ...}` | `S/ServerNotification.json` → `Account/rateLimits/updatedNotification` |
| 更新是稀疏更新；应合并可用字段到最近完整读取结果，或重新读取；更新中不可用的可空账号元数据不会清除旧值 | `S/v2/AccountRateLimitsUpdatedNotification.json` → `description` |
| 完整读取可带可空 `accountId`，是后端提供的快照关联账号；不是每次都保证提供 | `S/v2/GetAccountRateLimitsResponse.json` → `properties.accountId` |
| `ordinaryUsageAllowed` 为可选、可空布尔值；不能从百分比或重置时间推断恢复使用权限 | 同上 → `properties.ordinaryUsageAllowed.description` |

官方文档补充语义：`usedPercent` 表示窗口内已用比例，`windowDurationMins` 表示窗口分钟数，`resetsAt` 表示下一次重置的 Unix 秒时间戳。[官方额度字段说明](https://learn.chatgpt.com/docs/app-server#6-rate-limits-chatgpt)

因此建议在数值有效时以 `100 - usedPercent` 显示“剩余”；值缺失、超出 0–100 或无法判定窗口时显示未知。窗口按时长识别：300 分钟为 5h，10080 分钟为 1w；不能把 `primary`、`secondary` 固定绑定到这两个窗口。出现多个 bucket 或同长度多个窗口时需先确定选择规则，不能任意取第一个。`credits`、`individualLimit` 和 token 活动统计属于其他度量，不能代替两个窗口的剩余比例。

## 初始化与读取方案

本机 `S/v1/InitializeParams.json` 要求 `clientInfo.name`、`clientInfo.version`；`capabilities` 可选。`S/ClientNotification.json` 声明客户端发送 `initialized` 通知。额度方法在默认生成 schema 中存在，但这仍不等于运行时鉴权与网络读取已成功。

官方协议规定：默认 stdio 使用逐行 JSON，线上消息省略 `jsonrpc` 字段；每个连接先发送一次 `initialize`，等响应后发送 `initialized`，再调用其他方法。[官方协议与初始化](https://learn.chatgpt.com/docs/app-server#initialization)

已验证可用的最小消息序列：

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"herdr-quota","version":"0.1.0"}}}
{"method":"initialized"}
{"id":2,"method":"account/read","params":{"refreshToken":false}}
{"id":3,"method":"account/rateLimits/read","params":{"excludeResetCreditDetails":true}}
```

`excludeResetCreditDetails` 在本机 `S/v2/NullableGetAccountRateLimitsParams.json` 中说明为跳过单独的重置 credit 详情查询，适用于后台轮询。不要声明未实现的 `supportsLunaReserve` 能力。本次读取成功且未创建 thread 或启动模型 turn；未对资源消耗作额外测量。

建议初次完整读取，然后在通知到来时重新读取，避免错误合并稀疏字段。通知 schema 不承诺其他 CLI/桌面进程的额度变化必然及时推送到此连接，也未提供刷新时效保证。用户已确认每 60 秒读取额度；刷新失败保留旧值并标记“过期”，首次失败显示未知，检测到账号变化时立即丢弃旧账号缓存。实际刷新与跨进程账号跟随仍需实测。

## 当前账号的含义与边界

本机 `S/v2/GetAccountResponse.json` 的 `account` 可空，账号类型包括 `chatgpt`、`apiKey`、`amazonBedrock`、`gcp`；ChatGPT 账号带 `email`、`planType` 和可选 `workspaceName`。`requiresOpenaiAuth` 是独立字段，不能单凭它判断已登录或存在订阅额度。`S/v2/GetAccountParams.json` 说明 `refreshToken:true` 会请求主动刷新；上述序列显式使用 false。

`S/v2/AccountUpdatedNotification.json` 区分 Codex 托管的 ChatGPT OAuth 与外部认证模式；托管模式由 Codex 保存和刷新 token。官方将额度接口描述为 ChatGPT 额度。[官方账号接口说明](https://learn.chatgpt.com/docs/app-server#authentication-modes)

首期可定义为“读取本插件启动的 app-server 所使用的当前 Codex ChatGPT 账号”。**这是否等于用户在其他 Codex 窗口选择的账号，尚未证实。** schema 未给额度读取提供目标账号参数；`accountId` 也可能缺失，`account/updated` 只有认证模式与方案信息，不能据此唯一识别同方案账号切换。账号变化后应使旧缓存失效并重新读取；跨进程切换检测、相同方案切换与运行中 app-server 是否自动跟随，均需实测。

同版本官方源码进一步说明：`get_account` 调用 `read_account`，后者从 `auth_cached()` 获取认证；该读取路径本身没有显式执行认证存储 reload。因而不能把定时 `account/read(refreshToken:false)` 直接当作“强制跟随其他进程切换账号”的保证。[0.169.0 账号读取实现（247–340 行）](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/app-server/src/request_processors/account_processor/workspace_routing.rs#L247)

可行的首期方案是每轮轮询启动一个新的短生命周期 app-server，由 Codex 自行加载当前持久化认证；无需插件直接解析认证文件。该方案避开长期进程认证缓存，但是否跟随用户指定客户端的选择仍取决于该客户端是否共享并持久化同一认证配置，需在后续真实切换时验收。若采用长期进程，必须另行验证其认证重载机制。账号身份应比较内存中的 `userId` 与额度 `accountId`，不以 email/方案名称作唯一键，不把身份写入日志；任一变化时丢弃旧缓存。为减少切换竞态，可在额度读取前后读取账号，发现不一致就丢弃本轮并重试；同一用户切换 workspace 仍须借助额度 `accountId` 判断，不能仅比较 `userId`。

## 实施前仍需验证

- 已完成已登录环境的一次读取，确认 ChatGPT 账号及 300/10080 分钟窗口；尚需确认 bucket 选择政策、其他账号类型与空值行为。
- 核对未登录、非 ChatGPT 认证、授权失效、网络失败及缺少窗口时的错误行为；schema 不能证明错误码或具体文案。
- 比较 CLI 官方额度展示与读取结果，确认剩余方向、重置时间单位及账号/工作区归属。
- 验证其他 Codex 客户端产生用量时的通知覆盖，以及账号切换后的快照归属、缓存隔离和重新启动需求。

上表 windowDurationMins 来自真实快照；其余数值列已脱敏；JSON 仅是请求示例。在线文档可能领先于本机，字段兼容性以 0.169.0 生成 schema 为准。

## 首期实现验证

Rust debug 与 release 二进制均已通过真实 `refresh` 读取，并输出两个窗口的剩余进度条与倒计时。程序每轮启动短生命周期 app-server，完整读取后只保存归一化窗口，忽略本轮稀疏通知；后续变化由下一次轮询读取。

当前实现将 `userId`、email、workspace 显示名和方案元数据做 SHA-256 后作为缓存归属键，原值不落盘；缺少 userId 与 email 时不复用旧数据。只有验证为同一归属键后，额度读取失败才保留旧值；初始化或账号读取失败时清空额度。这个键不能证明同一用户下同名 workspace 的独立身份，也不能证明其他客户端使用相同登录存储；真实切换及更强的 workspace 归属校验仍属于未验收边界。接口返回的原始 `accountId` 不写入缓存。
