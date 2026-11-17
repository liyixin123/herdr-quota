use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn background_refresh_survives_status_command_group_cleanup() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let f = Fixture::new();
    let script = fs::read_to_string(&f.server).unwrap();
    fs::write(
        &f.server,
        script.replace("while IFS=", "sleep 0.2\nwhile IFS="),
    )
    .unwrap();
    // Herdr 0.9.5 kills the status command's process group even on success.
    let mut child = Command::new("/bin/sh")
        .env("CODEX_HOME", f.dir.join("codex-home"))
        .args([
            "-c",
            "\"$1\" status --cache-dir \"$2\" --codex \"$3\"; sleep 30",
            "sh",
        ])
        .arg(env!("CARGO_BIN_EXE_herdr-quota"))
        .arg(f.dir.join("cache"))
        .arg(&f.server)
        .process_group(0)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.contains("读取中"));
    assert!(
        Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .status()
            .unwrap()
            .success()
    );
    child.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(data) = fs::read(f.dir.join("cache/quota.json")) {
            let cache: Value = serde_json::from_slice(&data).unwrap();
            if cache["state"] == "ready" {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "background refresh was killed; status remains 读取中"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        String::from_utf8(f.run("status").stdout)
            .unwrap()
            .contains("72%")
    );
}

struct Fixture {
    dir: PathBuf,
    server: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "herdr-quota-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let server = dir.join("codex");
        let script = format!(
            r#"#!/bin/sh
while IFS= read -r line; do
  mode=$(cat '{}')
  case "$line" in
    *'"method":"initialize"'*) echo '{{"id":1,"result":{{}}}}' ;;
    *'"method":"account/read"'*)
      case "$mode" in
        logout) echo '{{"id":2,"result":{{"account":null}}}}' ;;
        api) echo '{{"id":2,"result":{{"account":{{"type":"apiKey"}}}}}}' ;;
        account_error) echo '{{"id":2,"error":{{"message":"PRIVATE_SENTINEL"}}}}' ;;
        switched_error) echo '{{"id":2,"result":{{"userId":"other-user","account":{{"type":"chatgpt","email":"other-PRIVATE_SENTINEL","planType":"plus"}}}}}}' ;;
        *) echo '{{"id":2,"result":{{"userId":"test-user","account":{{"type":"chatgpt","email":"PRIVATE_SENTINEL","planType":"plus"}}}}}}' ;;
      esac ;;
    *'"method":"account/rateLimits/read"'*)
      echo '{{"method":"account/rateLimits/updated","params":{{"rateLimits":{{"primary":null}}}}}}'
      case "$mode" in
        quota_error|switched_error) echo '{{"id":3,"error":{{"message":"PRIVATE_SENTINEL"}}}}' ;;
        *) cat '{}'; echo ;;
      esac ;;
  esac
done
"#,
            dir.join("mode").display(),
            dir.join("response").display()
        );
        fs::write(&server, script).unwrap();
        fs::set_permissions(&server, fs::Permissions::from_mode(0o700)).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fs::write(
            dir.join("response"),
            json!({"id":3,"result":{
                "accountId":"PRIVATE_SENTINEL",
                "rateLimits":{},
                "rateLimitsByLimitId":{"codex":{
                    "secondary":{"windowDurationMins":300,"usedPercent":28,"resetsAt":now+3600},
                    "primary":{"windowDurationMins":10080,"usedPercent":67,"resetsAt":now+86400}
                }}, "ordinaryUsageAllowed":true
            }})
            .to_string(),
        )
        .unwrap();
        fs::write(dir.join("mode"), "ok").unwrap();
        Self { dir, server }
    }

    fn run(&self, command: &str) -> Output {
        let output = Command::new(env!("CARGO_BIN_EXE_herdr-quota"))
            .env("CODEX_HOME", self.dir.join("codex-home"))
            .arg(command)
            .arg("--cache-dir")
            .arg(self.dir.join("cache"))
            .arg("--codex")
            .arg(&self.server)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_SENTINEL"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_SENTINEL"));
        output
    }

    fn next(&self, mode: &str) {
        fs::write(self.dir.join("mode"), mode).unwrap();
        let path = self.dir.join("cache/quota.json");
        let mut cache: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        cache["attempted_at"] = json!(0);
        fs::write(path, cache.to_string()).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn protocol_to_status_does_not_persist_identity_or_sparse_updates() {
    let f = Fixture::new();
    let output = f.run("refresh");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("72%"));
    assert!(text.contains("33%"));
    assert!(!text.contains("过期"));
    let cache = fs::read_to_string(f.dir.join("cache/quota.json")).unwrap();
    assert!(!cache.contains("PRIVATE_SENTINEL"));
    assert!(!cache.contains("test-user"));
    let mode = fs::metadata(f.dir.join("cache/quota.json"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let status = f.run("status");
    assert_eq!(status.stdout, text.as_bytes());
}

#[test]
fn failed_quota_read_keeps_verified_accounts_old_data() {
    let f = Fixture::new();
    f.run("refresh");
    f.next("quota_error");
    let output = f.run("refresh");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("72%"));
    assert!(text.contains("过期"));
    // A successful later refresh recovers without restarting Herdr.
    f.next("ok");
    assert!(
        !String::from_utf8(f.run("refresh").stdout)
            .unwrap()
            .contains("过期")
    );
}

#[test]
fn changed_or_unverifiable_account_never_reuses_old_quota() {
    for mode in ["switched_error", "account_error", "logout", "api"] {
        let f = Fixture::new();
        f.run("refresh");
        f.next(mode);
        let text = String::from_utf8(f.run("refresh").stdout).unwrap();
        assert!(!text.contains("72%"), "{mode}: {text}");
        assert!(text.contains("未知"));
        let cache: Value =
            serde_json::from_slice(&fs::read(f.dir.join("cache/quota.json")).unwrap()).unwrap();
        assert!(cache["quota"].is_null());
    }
}

#[test]
fn first_failure_is_unknown_and_attempts_are_rate_limited() {
    let f = Fixture::new();
    fs::write(f.dir.join("mode"), "quota_error").unwrap();
    let text = String::from_utf8(f.run("refresh").stdout).unwrap();
    assert!(text.contains("未知"));
    assert!(!text.contains("100%"));
    fs::write(f.dir.join("mode"), "ok").unwrap();
    assert_eq!(f.run("refresh").stdout, text.as_bytes());
}

#[test]
fn concurrent_refresh_has_single_owner() {
    let f = Fixture::new();
    let mut children = Vec::new();
    for _ in 0..4 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_herdr-quota"))
                .env("CODEX_HOME", f.dir.join("codex-home"))
                .arg("refresh")
                .arg("--cache-dir")
                .arg(f.dir.join("cache"))
                .arg("--codex")
                .arg(&f.server)
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let data = fs::read_to_string(f.dir.join("cache/quota.json")).unwrap();
    let value: Value = serde_json::from_str(&data).unwrap();
    assert_eq!(value["quota"]["five_hour"]["remaining"], 72);
}

#[test]
fn generated_command_handles_shell_metacharacters_in_paths() {
    let f = Fixture::new();
    f.run("refresh");
    let odd_path = f.dir.join("cache ' $(touch SHOULD_NOT_EXIST) `x`");
    fs::create_dir(&odd_path).unwrap();
    fs::copy(f.dir.join("cache/quota.json"), odd_path.join("quota.json")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_herdr-quota"))
        .env("CODEX_HOME", f.dir.join("codex-home"))
        .arg("herdr-config")
        .arg("--cache-dir")
        .arg(&odd_path)
        .arg("--codex")
        .arg(&f.server)
        .output()
        .unwrap();
    assert!(output.status.success());
    let snippet = String::from_utf8(output.stdout).unwrap();
    let encoded = snippet
        .lines()
        .find_map(|line| line.split_once("command = ").map(|(_, rest)| rest))
        .unwrap();
    let encoded = encoded.split(", interval_seconds").next().unwrap();
    let command: String = serde_json::from_str(encoded).unwrap();
    let status = Command::new("/bin/sh")
        .env("CODEX_HOME", f.dir.join("codex-home"))
        .arg("-c")
        .arg(command)
        .current_dir(&f.dir)
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(String::from_utf8(status.stdout).unwrap().contains("72%"));
    assert!(!f.dir.join("SHOULD_NOT_EXIST").exists());
}

#[test]
fn slow_starting_app_server_still_yields_quota() {
    // A cold Node.js `codex app-server` can stay silent for well over ten
    // seconds before its first reply. That start-up cost must not be reported
    // as a failed read, so the session budget has to exceed it.
    let f = Fixture::new();
    let script = fs::read_to_string(&f.server).unwrap();
    fs::write(
        &f.server,
        script.replace("while IFS=", "sleep 16\nwhile IFS="),
    )
    .unwrap();
    let text = String::from_utf8(f.run("refresh").stdout).unwrap();
    assert!(text.contains("72%"), "{text}");
}

#[test]
fn unresponsive_app_server_has_bounded_lifetime() {
    let f = Fixture::new();
    fs::write(
        &f.server,
        "#!/bin/sh\nwhile IFS= read -r line; do :; done\n",
    )
    .unwrap();
    let started = std::time::Instant::now();
    let text = String::from_utf8(f.run("refresh").stdout).unwrap();
    assert!(text.contains("未知"));
    // Bounded by the session budget (30s) with margin; a hung server must still
    // terminate instead of blocking the caller forever.
    assert!(started.elapsed() < std::time::Duration::from_secs(40));
}
