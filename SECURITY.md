# 安全政策

## 反馈渠道

发现安全问题时，请优先使用 GitHub 私有漏洞报告（Private Vulnerability Reporting），不要在公开 Issue 中报告安全细节：

- 仓库页 **Security → Report a vulnerability**，或直接访问 <https://github.com/liyixin123/herdr-quota/security/advisories/new>。

无法使用私有渠道时，请先通过上述入口或 Issue 留下联系方式，约定私密渠道后再发送细节。

## 报告时请避免包含凭据或账号信息

无论通过哪个渠道，请勿附带或粘贴：

- token、密码、API key 等凭据；
- 账号邮箱、账号 ID 等账号信息；
- Codex 会话内容或其他可定位个人账号的数据。

需要引用程序输出时，请先脱敏（README 中的额度示例均为示意数据，可作为格式参考）。公开 Issue、PR 与评论同样不要包含上述内容。

## 支持版本

仅对[最新 Release](https://github.com/liyixin123/herdr-quota/releases/latest) 提供安全修复。

## 维护者说明：私有漏洞报告启用状态

2026-11-17 尝试启用：

```sh
gh api repos/liyixin123/herdr-quota/private-vulnerability-reporting -X PUT
```

返回 403 `Resource not accessible by personal access token`——细粒度 PAT 无法调用该管理端点，与 [docs/ci.md](docs/ci.md) 中分支保护、安全扫描端点的限制同类。启用步骤（需经典 PAT 或网页端）：**Settings → Code security and analysis → Private vulnerability reporting → Enable**，或使用上面的 `gh api` 命令。启用后请更新本节，记录生效时间并确认报告入口可访问。
