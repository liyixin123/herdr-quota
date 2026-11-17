# Direct HTTP source evidence

Official source HEAD retrieved 2026-11-18: `ce5a565ba0f2e459e953bb18f1cedd5440f46d6b` (not necessarily same as installed CLI 0.169.0). Checked out at /tmp/codex-source-http.

- GET `${base}/wham/usage` for ChatGPT backend, `${base}/api/codex/usage` for Codex path style: [rate_limit_resets.rs](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client/rate_limit_resets.rs#L68-L85), [URL construction](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client/rate_limit_resets.rs#L128-L133). Passive usage readers do not opt into Luna reserve (lines 23-30).
- ChatGPT hosts normalize to `/backend-api`; path style detects `/backend-api`: [client.rs](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client.rs#L248-L260). Thus default ChatGPT URL is `https://chatgpt.com/backend-api/wham/usage`.
- Request builder resolves auth, applies auth headers, sends UA (fallback `codex-cli`) and optional `ChatGPT-Account-ID`; FedRAMP account can additionally require `X-OpenAI-Fedramp: true`: [headers](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client.rs#L375-L399). `CodexAuth::get_token` returns access_token for ChatGPT: [manager](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/manager.rs#L670-L680). Do not assert these are a permanent/public minimal HTTP contract.
- Wire payload is top-level `rate_limit` with primary/secondary window; additional_rate_limits is separate. Source normalizes default limit ID to codex: [mapping](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client.rs#L746-L777). Fields `used_percent`, `limit_window_seconds`, `reset_at`: [window mapping](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/client.rs#L880-L892). Read 18000 and 604800 second windows by duration, remaining = 100-used. Optional top-level account_id/user_id exist: [types](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/backend-client/src/types.rs#L58-L73).
- `$CODEX_HOME/auth.json` has optional auth_mode, OPENAI_API_KEY, tokens, last_refresh and newer auth variants: [storage](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/storage.rs#L51-L82). File not universal: storage implementation supports direct keyring, managed secrets and Auto fallback. In particular unavailable keyring cannot prove stale fallback is current: [storage](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/storage.rs#L210-L225). Prototype must explicitly scope file-mode, not silently trust arbitrary auth.json with keyring/auto.
- Workspace selection: retained ID token may describe old workspace until refresh, current account ID lives in tokens.account_id; source reconciles claims with access token: [workspace_switching](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/manager/workspace_switching.rs#L354-L367). Do not choose account from stale ID token.
- Refresh endpoint `https://auth.openai.com/oauth/token`; source has explicit reused/expired/revoked failures: [constants](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/manager.rs#L232-L243). Guarded refresh reloads expected account and skips if changed: [guarded refresh](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/manager.rs#L3474-L3520). Persistence uses write lock and compare-and-save of expected access+refresh token and account identity: [persistence](https://github.com/openai/codex/blob/ce5a565ba0f2e459e953bb18f1cedd5440f46d6b/codex-rs/login/src/auth/manager.rs#L1759-L1804). Inference: independently refreshing without full coordination risks rotation races. Safe prototype should never refresh/write credentials; on 401 report auth rejected and leave recovery to Codex.

Source-level assessment: the request shape supports ordinary file-backed ChatGPT OAuth tokens; the live result and remaining production gates are recorded below. Endpoint is first-party internal backend source, not demonstrated public API contract; do not promise compatibility.

## Prototype and reproduction

The isolated experiment is `experiments/http-quota/probe.py` (Python standard library, Linux/macOS; Unix interval timer). It is not imported by the Rust application and does not alter the production query path. Python was selected to test the protocol without adding production dependencies; its memory consumption is not a prediction for a future Rust implementation.

Run from the repository root:

```sh
python3 -m unittest discover -s experiments/http-quota -p 'test_*.py'
python3 experiments/http-quota/probe.py --file-auth
python3 experiments/http-quota/measure.py
python3 experiments/http-quota/measure_server.py
```

`--file-auth` is an explicit acknowledgment that the selected auth.json is the active file credential store. The experiment does not resolve layered Codex configuration, managed auth, keyring/auto, profiles, custom backend URLs, or FedRAMP accounts. It must not be presented as universally reading the current account. The file defaults to `$CODEX_HOME/auth.json` or `~/.codex/auth.json`; an explicit `--auth-file` is available. The prototype uses `tokens.account_id`, never stale ID-token workspace claims. A before/after fingerprint discards results if the auth file changes during a query (including a token rotation); it cannot prove a stale file matches a different active credential store or detect an ABA switch.

Only GET `https://chatgpt.com/backend-api/wham/usage` is used. Environment HTTPS proxy configuration is honored; system TLS certificate verification is retained. Redirects are rejected before any second request. The hidden fixture endpoint allows only literal loopback `127.0.0.1` and fixed public fixture credentials; it rejects real credentials before network access. No server response body, auth path contents, account ID, or token is printed; output is a fixed status or supported quota windows. Nothing is written to the credential store. The response is limited to 1 MiB and a Unix timer bounds the entire query at 10 seconds by default (maximum 30).

## Behavior validation

The CLI subprocess tests exercise a local HTTP server with synthetic credentials:

| Case | Observed behavior |
| --- | --- |
| 200 with 5h/1w windows | Remaining = 100-used; windows selected by duration; unrelated response fields omitted |
| Missing credentials | `logged_out_or_unsupported_store`, no request |
| Real credentials offered to fixture endpoint | `unsafe_fixture`, no request |
| 401 | `auth_expired_or_rejected`; no refresh call or credential write |
| Account/token change during successful query | `account_changed_during_query`; quota discarded |
| Redirect | `http_302`; exactly one request, no credential forwarding |
| Slow network | `network_timeout`; bounded CLI exit |

A 401 is rejection, not proof the token expired; production must avoid misleading attribution. Other HTTP statuses are reported as fixed `http_NNN` states; transport errors become `network_error`. Real refresh-token expiry/revocation was not induced. The tests verify local handling of a 401, not the server's entire expiry protocol. Rotating credentials or changing the user's live account solely for this experiment was avoided.

## Live observations and measurement limitations (2026-11-18)

Environment: Linux 6.6.87.2 WSL2 x86_64, glibc 2.44, Python 3.14, 12 logical CPUs; installed `codex-cli 0.169.0`. Official source evidence above is pinned to a different HEAD, so the two versions must not be equated. The local config did not explicitly set `cli_auth_credentials_store`; local auth.json had a ChatGPT token bundle. The live result proves that token bundle is accepted, not that every auth configuration is supported.

Direct HTTP succeeded through the environment HTTPS proxy, returning both 5h/1w windows. During three consecutive measured runs the normalized windows were 68% and 95% remaining, with unchanged reset timestamps. Earlier 69% changed to 68% while the research session was active; compare snapshots close in time rather than assuming static usage. Raw response bodies and credentials were not saved.

| Direct HTTP run | Wall seconds (process + import + TLS + request) | User+system CPU seconds | Peak RSS MiB |
| --- | ---: | ---: | ---: |
| 1 | 1.7994 | 0.0855 | 26.37 |
| 2 | 0.7304 | 0.0850 | 26.24 |
| 3 | 0.7634 | 0.0859 | 26.35 |

`measure.py` uses a fresh worker per run and `getrusage(RUSAGE_CHILDREN)` after the CLI exits; Linux peak RSS is converted from KiB. Numbers include the Python interpreter and all query work, exclude the benchmark supervisor, and do not isolate TLS/network time from startup. They are single-machine observations, not service guarantees.

A live app-server comparison could not be completed: the normal startup exited with read-only filesystem errors in this sandbox. Redirecting sqlite/log directories into a temporary directory still failed to answer initialization within 30 seconds; it was terminated. Therefore live snapshot equivalence and authenticated app-server CPU/memory savings remain **unverified**, and no percentage saving is claimed. No model turn was started and no credentials were copied to temporary directories. An isolated unauthenticated app-server baseline is measured separately below; it is not a paired authenticated quota-query comparison.

The server measurement bounds the complete initialize exchange (including partial output lines) with a Unix interval timer, defaults to 30 seconds, and caps its response read at 1 MiB. A fake server regression verifies timeout after a partial line with a 0.1-second budget.

Unauthenticated baseline from `measure_server.py`, using an empty temporary CODEX_HOME and only initialize/initialized:

- Initialization: 0.2003 seconds.
- At 2 seconds after initialization: 4 processes, summed RSS 97.87 MiB, cumulative visible CPU 0.12 seconds.
- After another 15 seconds: 1 process, RSS 78.50 MiB, cumulative visible CPU 0.14 seconds.
- Visible CPU delta / elapsed time: approximately 0.133% of one core. Child exit can remove CPU from these `/proc` snapshots, so this is an indicative idle sample, not exact total process-tree CPU accounting.

RSS here is the sum of live process RSS snapshots, whereas the direct-query metric is peak child RSS from getrusage. Different lifetimes, authentication and metric definitions prevent a valid speedup or memory-reduction ratio. The baseline supports the earlier ~80 MiB idle observation, but the direct prototype is short-lived and retains no resident service.

## Decision and next implementation boundary

**Direct HTTP is feasible for an accepted file-backed ChatGPT token bundle:** the real request returned quota successfully and the protocol has first-party source evidence. Recommend pursuing an opt-in production implementation only after defining the authoritative credential-store selection and recovery policy. Do not replace the existing production path based on this experiment alone.

A follow-up implementation should resolve current account identity using the effective Codex auth-store configuration, preserve existing cache identity/expiry behavior, use a Rust HTTP client with equivalent proxy/TLS/redirect/timeout protections, and define recovery for rejected/expired tokens without racing Codex's refresh rotation. Keyring/auto/managed and custom/FedRAMP backends require explicit support or a clear unsupported state. Re-run paired authenticated snapshots and benchmarks in a writable environment before claiming parity or performance gains. If these auth requirements outweigh the benefit, retain the app-server method; the endpoint is internal and can change independently of this plugin.

This ticket delivers the read-only experiment, reproducible evidence, failure tests and a conditional adoption conclusion. Production integration, token refresh, authoritative credential-store support and a paired authenticated benchmark remain follow-up work, not hidden completed claims.
