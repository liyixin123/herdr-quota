//! Read-only fast path for the ordinary file-backed ChatGPT login.
//! Unsupported configuration and all request failures delegate to Codex itself.
use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::model::{Quota, normalize};

const ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";
const MAX_BYTES: u64 = 1_048_576;

fn bounded_read(path: &Path) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| ())?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(());
    }
    Ok(bytes)
}

fn config_allows_file(home: &Path) -> bool {
    let config = match bounded_read(&home.join("config.toml")) {
        Ok(bytes) => match String::from_utf8(bytes)
            .ok()
            .and_then(|s| s.parse::<toml_edit::DocumentMut>().ok())
        {
            Some(config) => config,
            None => return false,
        },
        Err(()) if !home.join("config.toml").try_exists().unwrap_or(true) => return true,
        Err(()) => return false,
    };
    // Codex's AuthCredentialsStoreMode defaults to File (not MCP's Auto).
    if config
        .get("cli_auth_credentials_store")
        .is_some_and(|v| v.as_str() != Some("file"))
    {
        return false;
    }
    // We deliberately do not partially implement profile/managed routing.
    for key in [
        "profile",
        "forced_login_method",
        "forced_chatgpt_workspace_id",
        "auth_provider",
        "runtime_auth_mode",
    ] {
        if config.contains_key(key) {
            return false;
        }
    }
    if config.get("chatgpt_base_url").is_some_and(|v| {
        !matches!(
            v.as_str(),
            Some("https://chatgpt.com") | Some("https://chatgpt.com/backend-api")
        )
    }) {
        return false;
    }
    true
}

fn has_config_layers(home: &Path, cwd: &Path) -> bool {
    // The user's primary ~/.codex/config.toml is also encountered while
    // walking repository ancestors below HOME; it is not a project override.
    let primary = home.join("config.toml");
    let primary_canonical = primary.canonicalize().ok();
    if cwd.ancestors().any(|dir| {
        let path = dir.join(".codex/config.toml");
        let is_primary = path == primary
            || primary_canonical
                .as_ref()
                .is_some_and(|p| path.canonicalize().ok().as_ref() == Some(p));
        !is_primary && path.try_exists().unwrap_or(true)
    }) {
        return true;
    }
    let paths = [
        PathBuf::from("/etc/codex/config.toml"),
        PathBuf::from("/etc/codex/managed_config.toml"),
        PathBuf::from("/etc/codex/requirements.toml"),
        home.join("managed_config.toml"),
        home.join("requirements.toml"),
    ];
    if paths.iter().any(|path| path.try_exists().unwrap_or(true)) {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        if Path::new("/Library/Managed Preferences/com.openai.codex.plist")
            .try_exists()
            .unwrap_or(true)
        {
            return true;
        }
        if let Some(user_home) = std::env::var_os("HOME") {
            if PathBuf::from(user_home)
                .join("Library/Managed Preferences/com.openai.codex.plist")
                .try_exists()
                .unwrap_or(true)
            {
                return true;
            }
        }
    }
    false
}

struct Candidate {
    home: PathBuf,
    fingerprint: Vec<u8>,
    access: String,
    account: String,
    identity: String,
}

impl Candidate {
    fn load(home: &Path) -> Result<Self, ()> {
        if !config_allows_file(home) {
            return Err(());
        }
        let bytes = bounded_read(&home.join("auth.json"))?;
        let data: Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if !matches!(
            data.get("auth_mode").and_then(Value::as_str),
            None | Some("chatgpt")
        ) {
            return Err(());
        }
        if data.get("OPENAI_API_KEY").is_some_and(|v| !v.is_null()) {
            return Err(());
        }
        let token = &data["tokens"];
        let access = token["access_token"].as_str().ok_or(())?;
        let account = token["account_id"].as_str().ok_or(())?;
        if [access, account]
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| (33..=126).contains(&b)))
        {
            return Err(());
        }
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(access.split('.').nth(1).ok_or(())?)
            .map_err(|_| ())?;
        let claims: Value = serde_json::from_slice(&claims).map_err(|_| ())?;
        let user = claims["sub"].as_str().filter(|s| !s.is_empty()).ok_or(())?;
        let expires = claims["exp"].as_u64().ok_or(())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())?
            .as_secs();
        if expires <= now.saturating_add(30) {
            return Err(());
        }
        if claims["https://api.openai.com/auth"]["chatgpt_account_is_fedramp"].as_bool()
            == Some(true)
        {
            return Err(());
        }
        if let Some(id_token) = token["id_token"].as_str() {
            let id_claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(id_token.split('.').nth(1).ok_or(())?)
                .map_err(|_| ())?;
            let id_claims: Value = serde_json::from_slice(&id_claims).map_err(|_| ())?;
            if id_claims["https://api.openai.com/auth"]["chatgpt_account_is_fedramp"].as_bool()
                == Some(true)
            {
                return Err(());
            }
        }
        let identity_bytes = serde_json::to_vec(&json!([account, user])).map_err(|_| ())?;
        let identity = format!("http:{}", crate::sha256_hex(&identity_bytes));
        Ok(Self {
            home: home.into(),
            fingerprint: Sha256::digest(&bytes).to_vec(),
            access: access.into(),
            account: account.into(),
            identity,
        })
    }

    fn read(self, agent: &ureq::Agent, endpoint: &str) -> Result<(String, Quota), ()> {
        let mut response = agent
            .get(endpoint)
            .header("Authorization", format!("Bearer {}", self.access))
            .header("ChatGPT-Account-Id", &self.account)
            .header("Accept", "application/json")
            .call()
            .map_err(|_| ())?;
        if response.status().as_u16() != 200 {
            return Err(());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(());
        }
        let response: Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if response
            .get("account_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id != self.account)
        {
            return Err(());
        }
        let rate_limit = response
            .get("rate_limit")
            .filter(|v| v.is_object())
            .ok_or(())?;
        let convert = |key: &str| {
            let window = &rate_limit[key];
            let duration = window["limit_window_seconds"].as_u64();
            json!({"windowDurationMins": duration.filter(|n| n % 60 == 0).map(|n| n / 60), "usedPercent":window["used_percent"], "resetsAt":window["reset_at"]})
        };
        let quota = normalize(&json!({"rateLimits":{"primary":convert("primary_window"),"secondary":convert("secondary_window")},"ordinaryUsageAllowed":rate_limit["allowed"]})).map_err(|_| ())?;
        if quota.five_hour.remaining.is_none() && quota.weekly.remaining.is_none() {
            return Err(());
        }
        if Candidate::load(&self.home)?.fingerprint != self.fingerprint {
            return Err(());
        }
        Ok((self.identity, quota))
    }
}

fn agent(env_proxy: bool) -> ureq::Agent {
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(0)
        .user_agent(concat!("herdr-quota/", env!("CARGO_PKG_VERSION")))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        );
    if !env_proxy {
        config = config.proxy(None);
    }
    config.build().into()
}

fn resolve_executable(executable: &Path, search_path: &std::ffi::OsStr) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    if executable.components().count() > 1 {
        return executable.canonicalize().ok();
    }
    std::env::split_paths(search_path)
        .map(|dir| dir.join(executable))
        .find(|path| {
            std::fs::metadata(path)
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })?
        .canonicalize()
        .ok()
}

fn same_default_executable(executable: &Path, search_path: &std::ffi::OsStr) -> bool {
    match (
        resolve_executable(executable, search_path),
        resolve_executable(Path::new("codex"), search_path),
    ) {
        (Some(selected), Some(default)) => selected == default,
        _ => false,
    }
}

pub fn read(executable: &Path) -> Result<(String, Quota), ()> {
    // An alternate wrapper may select a different CODEX_HOME or credentials.
    if !same_default_executable(executable, &std::env::var_os("PATH").ok_or(())?) {
        return Err(());
    }
    // Other auth routes can override the file even when old tokens remain.
    for name in [
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "CODEX_PROFILE",
        "CODEX_AUTH_MODE",
        "CODEX_MANAGED_CONFIG",
        "CHATGPT_BASE_URL",
    ] {
        if std::env::var_os(name).is_some() {
            return Err(());
        }
    }
    let home = match std::env::var_os("CODEX_HOME") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return Err(()),
        None => PathBuf::from(std::env::var_os("HOME").ok_or(())?).join(".codex"),
    };
    let cwd = std::env::current_dir().map_err(|_| ())?;
    if has_config_layers(&home, &cwd) {
        return Err(());
    }
    let result = Candidate::load(&home)?.read(&agent(true), ENDPOINT)?;
    // A newly introduced config layer during the request also invalidates it.
    if has_config_layers(&home, &cwd) {
        return Err(());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "herdr-http-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(br#"{"sub":"fixture-user","exp":4102444800}"#);
            std::fs::write(dir.join("auth.json"), serde_json::to_vec(&serde_json::json!({"auth_mode":"chatgpt","tokens":{"access_token":format!("e30.{claims}.fixture"),"account_id":"fixture-account"}})).unwrap()).unwrap();
            Self(dir)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn file_account_reads_quota_without_persisting_credentials() {
        let fixture = Fixture::new();
        let before = std::fs::read(fixture.0.join("auth.json")).unwrap();
        let candidate = Candidate::load(&fixture.0).unwrap();
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/usage", server.local_addr().unwrap());
        let handler = std::thread::spawn(move || {
            let (mut stream, _) = server.accept().unwrap();
            let mut request = [0; 8192];
            let size = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(
                request
                    .to_lowercase()
                    .contains("chatgpt-account-id: fixture-account")
            );
            let body = r#"{"rate_limit":{"primary_window":{"used_percent":28,"limit_window_seconds":18000,"reset_at":2000000000},"secondary_window":{"used_percent":10,"limit_window_seconds":604800,"reset_at":2000000100}}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let agent = agent(false);
        let (identity, quota) = candidate.read(&agent, &endpoint).unwrap();
        assert_eq!(quota.five_hour.remaining, Some(72));
        assert_eq!(quota.weekly.remaining, Some(90));
        assert!(!identity.contains("fixture"));
        assert_eq!(std::fs::read(fixture.0.join("auth.json")).unwrap(), before);
        handler.join().unwrap();
    }
    #[test]
    fn ambiguous_stores_and_expired_tokens_delegate_without_changing_auth() {
        let fixture = Fixture::new();
        let original = std::fs::read(fixture.0.join("auth.json")).unwrap();
        for config in [
            "cli_auth_credentials_store='keyring'",
            "cli_auth_credentials_store='auto'",
            "cli_auth_credentials_store='ephemeral'",
            "profile='work'",
            "chatgpt_base_url='https://example.invalid'",
            "not valid toml",
        ] {
            std::fs::write(fixture.0.join("config.toml"), config).unwrap();
            assert!(Candidate::load(&fixture.0).is_err(), "{config}");
            assert_eq!(
                std::fs::read(fixture.0.join("auth.json")).unwrap(),
                original
            );
        }
        std::fs::write(
            fixture.0.join("config.toml"),
            "cli_auth_credentials_store='file'",
        )
        .unwrap();
        assert!(Candidate::load(&fixture.0).is_ok());
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"sub":"fixture-user","exp":1}"#);
        std::fs::write(fixture.0.join("auth.json"), serde_json::to_vec(&json!({"tokens":{"access_token":format!("e30.{claims}.fixture"),"account_id":"fixture-account"}})).unwrap()).unwrap();
        assert!(Candidate::load(&fixture.0).is_err());
    }

    #[test]
    fn failed_redirected_and_changed_account_queries_discard_results() {
        for mode in [
            "unauthorized",
            "redirect",
            "switch",
            "oversized",
            "wrong_account",
            "timeout",
        ] {
            let fixture = Fixture::new();
            let candidate = Candidate::load(&fixture.0).unwrap();
            let before = std::fs::read(fixture.0.join("auth.json")).unwrap();
            let auth = fixture.0.join("auth.json");
            let server = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/usage", server.local_addr().unwrap());
            let location = endpoint.clone();
            let handler = std::thread::spawn(move || {
                let (mut stream, _) = server.accept().unwrap();
                let _ = stream.read(&mut [0; 8192]);
                if mode == "timeout" {
                    std::thread::sleep(Duration::from_millis(150));
                    return;
                }
                let status = match mode {
                    "unauthorized" => "401 Unauthorized",
                    "redirect" => "302 Found",
                    _ => "200 OK",
                };
                if mode == "switch" {
                    std::fs::write(auth, b"{}").unwrap();
                }
                let body = if mode == "oversized" {
                    "x".repeat(MAX_BYTES as usize + 1)
                } else if mode == "wrong_account" {
                    r#"{"account_id":"other-account","rate_limit":{"primary_window":{"used_percent":28,"limit_window_seconds":18000,"reset_at":2000000000}}}"#.into()
                } else {
                    r#"{"rate_limit":{"primary_window":{"used_percent":28,"limit_window_seconds":18000,"reset_at":2000000000}}}"#.into()
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: {location}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            });
            let mut config = ureq::Agent::config_builder().proxy(None).max_redirects(0);
            if mode == "timeout" {
                config = config.timeout_global(Some(Duration::from_millis(50)));
            }
            let agent: ureq::Agent = config.build().into();
            assert!(candidate.read(&agent, &endpoint).is_err(), "{mode}");
            handler.join().unwrap();
            if mode != "switch" {
                assert_eq!(std::fs::read(fixture.0.join("auth.json")).unwrap(), before);
            }
        }
    }

    #[test]
    fn project_configuration_prevents_guessing_the_credential_store() {
        let fixture = Fixture::new();
        let child = fixture.0.join("project/nested");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir_all(fixture.0.join("project/.codex")).unwrap();
        std::fs::write(
            fixture.0.join("project/.codex/config.toml"),
            "cli_auth_credentials_store='keyring'",
        )
        .unwrap();
        assert!(has_config_layers(&fixture.0, &child));
    }

    #[test]
    fn alternate_codex_wrapper_preserves_app_server_auth_selection() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let fixture = Fixture::new();
        let default = fixture.0.join("codex");
        let alternate = fixture.0.join("alternate-codex");
        for file in [&default, &alternate] {
            std::fs::write(file, "#!/bin/sh\nexit 1\n").unwrap();
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let search = fixture.0.as_os_str();
        assert!(same_default_executable(&default, search));
        assert!(!same_default_executable(&alternate, search));
        symlink(&default, fixture.0.join("alias-codex")).unwrap();
        assert!(same_default_executable(
            &fixture.0.join("alias-codex"),
            search
        ));
    }
    #[test]
    fn primary_home_configuration_is_not_a_project_override() {
        let fixture = Fixture::new();
        let home = fixture.0.join("home/.codex");
        let cwd = fixture.0.join("home/repo/project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(
            home.join("config.toml"),
            "cli_auth_credentials_store='file'",
        )
        .unwrap();
        assert!(!has_config_layers(&home, &cwd));
        assert!(config_allows_file(&home));
    }
}
