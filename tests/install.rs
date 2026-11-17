use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "hq-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        for (name, script) in [
            ("herdr", "#!/bin/sh\n[ ! -f \"$HOME/stopped\" ]\n"),
            (
                "codex",
                r#"#!/bin/sh
while IFS= read -r line; do
case "$line" in
*'"method":"initialize"'*) echo '{"id":1,"result":{}}';;
*'"method":"account/read"'*) echo '{"id":2,"result":{"account":{"type":"chatgpt","email":"test","planType":"plus"}}}';;
esac
done
"#,
            ),
        ] {
            let p = dir.join(name);
            fs::write(&p, script).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self(dir)
    }
    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_herdr-quota"))
            .arg("install")
            .arg("--install-dir")
            .arg(self.0.join("bin"))
            .arg("--config")
            .arg(self.0.join("config.toml"))
            .arg("--cache-dir")
            .arg(self.0.join("cache"))
            .arg("--codex")
            .arg(self.0.join("codex"))
            .arg("--herdr")
            .arg(self.0.join("herdr"))
            .env("HOME", &self.0)
            .env("XDG_DATA_HOME", self.0.join("data"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn install_creates_configuration_and_installs_absolute_binary() {
    let f = Fixture::new();
    let out = f.run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let config = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(config.contains("hide_tab_bar_when_single_tab = false"));
    assert!(config.contains(f.0.join("bin/herdr-quota").to_str().unwrap()));
    assert!(config.contains(f.0.join("codex").to_str().unwrap()));
    assert!(f.0.join("bin/herdr-quota").is_file());
    assert!(String::from_utf8_lossy(&out.stdout).contains("reloaded"));
}

#[test]
fn install_preserves_existing_status_and_position_and_is_repeatable() {
    let f = Fixture::new();
    let original = "# keep my comments\n[ui]\ntab_bar_position = \"bottom\"\nhide_tab_bar_when_single_tab = true\ntab_bar_right = [{ type = \"hostname\" }]\n[terminal]\nfont_size = 12\n";
    fs::write(f.0.join("config.toml"), original).unwrap();
    let out = f.run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let first = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(first.contains("# keep my comments"));
    assert!(first.contains(r#"tab_bar_position = "bottom""#));
    assert!(first.contains(r#"type = "hostname""#));
    assert!(first.contains("font_size = 12"));
    let backup = fs::read_dir(&f.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_str().unwrap().contains("backup"))
        .unwrap();
    assert_eq!(fs::read_to_string(backup).unwrap(), original);
    assert!(f.run().status.success());
    assert_eq!(fs::read_to_string(f.0.join("config.toml")).unwrap(), first);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join("data/herdr-quota/install.json")).unwrap())
            .unwrap();
    assert_eq!(state["previous_hide"], " true");
}

#[test]
fn just_install_builds_and_connects_with_isolated_dependencies() {
    let f = Fixture::new();
    let cargo = f.0.join("cargo");
    fs::write(&cargo, "#!/bin/sh\n[ \"$*\" = 'build --release --locked' ] || exit 2\nmkdir -p \"$CARGO_TARGET_DIR/release\"\ncp \"$TEST_BINARY\" \"$CARGO_TARGET_DIR/release/herdr-quota\"\n").unwrap();
    fs::set_permissions(cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new("just")
        .arg("install")
        .env(
            "PATH",
            format!("{}:{}", f.0.display(), std::env::var("PATH").unwrap()),
        )
        .env("HOME", &f.0)
        .env("XDG_CONFIG_HOME", f.0.join("config"))
        .env_remove("HERDR_CONFIG_PATH")
        .env("XDG_DATA_HOME", f.0.join("data"))
        .env("CARGO_TARGET_DIR", f.0.join("target"))
        .env("TEST_BINARY", env!("CARGO_BIN_EXE_herdr-quota"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(f.0.join(".local/bin/herdr-quota").is_file());
    assert!(f.0.join("config/herdr/config.toml").is_file());
}

#[test]
fn install_accepts_array_of_table_status_configuration() {
    let f = Fixture::new();
    fs::write(
        f.0.join("config.toml"),
        "[ui]\ntab_bar_position = 'bottom'\n[[ui.tab_bar_right]]\ntype = 'hostname'\n",
    )
    .unwrap();
    let output = f.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(text.contains("type = 'hostname'"));
    assert!(text.contains("# herdr-quota"));
    assert!(f.run().status.success());
    assert_eq!(text, fs::read_to_string(f.0.join("config.toml")).unwrap());
}

#[test]
fn failed_reload_can_be_retried_without_duplicate_configuration() {
    let f = Fixture::new();
    fs::write(f.0.join("stopped"), "").unwrap();
    let output = f.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("start the local Herdr"));
    let first = fs::read_to_string(f.0.join("config.toml")).unwrap();
    fs::remove_file(f.0.join("stopped")).unwrap();
    assert!(f.run().status.success());
    assert_eq!(fs::read_to_string(f.0.join("config.toml")).unwrap(), first);
}

#[test]
fn malformed_configuration_is_retained_and_repaired_on_retry() {
    let f = Fixture::new();
    fs::write(f.0.join("config.toml"), "[broken\n").unwrap();
    let output = f.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("repair TOML and rerun"));
    assert_eq!(
        fs::read_to_string(f.0.join("config.toml")).unwrap(),
        "[broken\n"
    );
    assert!(!f.0.join("bin/herdr-quota").exists());
    fs::write(f.0.join("config.toml"), "[ui]\n").unwrap();
    assert!(f.run().status.success());
}

#[test]
fn prerequisites_have_repair_guidance_and_do_not_modify_config() {
    for missing in ["codex", "herdr"] {
        let f = Fixture::new();
        fs::remove_file(f.0.join(missing)).unwrap();
        let output = f.run();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("rerun"));
        assert!(!f.0.join("config.toml").exists());
        assert!(!f.0.join("bin/herdr-quota").exists());
    }
    let f = Fixture::new();
    fs::write(f.0.join("codex"), "#!/bin/sh\nwhile IFS= read -r line; do\ncase \"$line\" in\n*'\"method\":\"initialize\"'*) echo '{\"id\":1,\"result\":{}}';;\n*'\"method\":\"account/read\"'*) echo '{\"id\":2,\"result\":{\"account\":null}}';;\nesac\ndone\n").unwrap();
    let output = f.run();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("codex login"));
    assert!(!f.0.join("config.toml").exists());
}

#[test]
fn configuration_write_failure_retains_original_and_retries_after_repair() {
    let f = Fixture::new();
    // A directory at the destination is a deterministic write/read failure even as root.
    fs::create_dir(f.0.join("config.toml")).unwrap();
    assert!(!f.run().status.success());
    assert!(f.0.join("config.toml").is_dir());
    fs::remove_dir(f.0.join("config.toml")).unwrap();
    assert!(f.run().status.success());
}
