use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs,
    path::{Path, PathBuf},
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
        self.command("install")
    }
    fn command(&self, command: &str) -> Output {
        self.cli(command, Path::new(env!("CARGO_BIN_EXE_herdr-quota")))
            .output()
            .unwrap()
    }
    fn cli(&self, command: &str, binary: &Path) -> Command {
        let mut cli = Command::new("sh");
        cli.args([
            "-c",
            r#"umask 077; exec "$@""#,
            "herdr-quota-test",
            binary.to_str().unwrap(),
        ])
        .arg(command)
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
        .env("XDG_DATA_HOME", self.0.join("data"));
        cli
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
    assert!(f.command("uninstall").status.success());
    let cleaned = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(cleaned.contains("type = 'hostname'"));
    assert!(!cleaned.contains("# herdr-quota"));
    assert!(!cleaned.contains("hide_tab_bar_when_single_tab"));
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

#[test]
fn uninstall_preserves_user_edits_and_other_status_items() {
    let f = Fixture::new();
    fs::write(f.0.join("config.toml"), "# user comment\n[ui]\nhide_tab_bar_when_single_tab = true\ntab_bar_right = [{type='hostname'}]\n[terminal]\nfont_size = 12\n").unwrap();
    assert!(f.run().status.success());
    let installed = fs::read_to_string(f.0.join("config.toml")).unwrap();
    fs::write(
        f.0.join("config.toml"),
        installed.replace("font_size = 12", "font_size = 18"),
    )
    .unwrap();
    let output = f.command("uninstall");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cleaned = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(cleaned.contains("# user comment"));
    assert!(cleaned.contains("type='hostname'"));
    assert!(cleaned.contains("font_size = 18"));
    assert!(cleaned.contains("hide_tab_bar_when_single_tab = true"));
    assert!(!cleaned.contains("# herdr-quota"));
    assert!(!f.0.join("bin/herdr-quota").exists());
    assert!(!f.0.join("data/herdr-quota/install.json").exists());
    assert!(f.command("uninstall").status.success());
}

#[test]
fn uninstall_retains_recovery_on_reload_failure_and_finishes_after_retry() {
    let f = Fixture::new();
    assert!(f.run().status.success());
    fs::write(f.0.join("stopped"), "").unwrap();
    let output = f.command("uninstall");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("rerun uninstall"));
    assert!(f.0.join("bin/herdr-quota").exists());
    assert!(f.0.join("data/herdr-quota/install.json").exists());
    assert!(
        !fs::read_to_string(f.0.join("config.toml"))
            .unwrap()
            .contains("# herdr-quota")
    );
    fs::remove_file(f.0.join("stopped")).unwrap();
    fs::remove_file(f.0.join("bin/herdr-quota")).unwrap();
    assert!(f.command("uninstall").status.success());
    assert!(!f.0.join("data/herdr-quota/install.json").exists());
}

#[test]
fn uninstall_reports_stopped_server_and_cleanup_effective_on_next_start() {
    let f = Fixture::new();
    assert!(f.run().status.success());
    fs::write(f.0.join("herdr"), "#!/bin/sh\nif [ \"$1\" = status ]; then echo '{\"server\":{\"status\":\"not_running\",\"running\":false}}'; exit 0; fi\nexit 1\n").unwrap();
    let output = f.command("uninstall");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("next start"));
    assert!(!f.0.join("bin/herdr-quota").exists());
}

#[test]
fn just_uninstall_builds_even_when_installed_program_was_manually_removed() {
    let f = Fixture::new();
    assert!(f.run().status.success());
    fs::remove_file(f.0.join("bin/herdr-quota")).unwrap();
    fs::remove_file(f.0.join("codex")).unwrap();
    let cargo = f.0.join("cargo");
    fs::write(&cargo, "#!/bin/sh\n[ \"$*\" = 'build --release --locked' ] || exit 2\nmkdir -p \"$CARGO_TARGET_DIR/release\"\ncp \"$TEST_BINARY\" \"$CARGO_TARGET_DIR/release/herdr-quota\"\n").unwrap();
    fs::set_permissions(cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new("just")
        .arg("uninstall")
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
    assert!(!f.0.join(".local/bin/herdr-quota").exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Uninstallation complete"));
}

#[test]
fn uninstall_without_state_removes_only_marked_entries_and_preserves_user_tab_setting() {
    let f = Fixture::new();
    fs::write(f.0.join("config.toml"), "[ui]\nhide_tab_bar_when_single_tab = true\n[[ui.tab_bar_right]]\ntype = 'command'\ncommand = 'echo custom herdr-quota'\n[[ui.tab_bar_right]]\ntype = 'command'\ncommand = 'old-path status # herdr-quota'\n").unwrap();
    let output = f.command("uninstall");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(f.0.join("config.toml")).unwrap();
    assert!(text.contains("echo custom herdr-quota"));
    assert!(text.contains("hide_tab_bar_when_single_tab = true"));
    assert!(!text.contains("old-path"));
    let backups: Vec<_> = fs::read_dir(&f.0)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("uninstall-backup")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        fs::metadata(&backups[0]).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        fs::read_to_string(&backups[0])
            .unwrap()
            .contains("old-path")
    );
}

#[test]
fn malformed_uninstall_config_preserves_program_and_state_for_repair() {
    let f = Fixture::new();
    assert!(f.run().status.success());
    fs::write(f.0.join("config.toml"), "[broken\n").unwrap();
    let output = f.command("uninstall");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("repair TOML and rerun uninstall"));
    assert!(f.0.join("bin/herdr-quota").exists());
    assert!(f.0.join("data/herdr-quota/install.json").exists());
    assert_eq!(
        fs::read_to_string(f.0.join("config.toml")).unwrap(),
        "[broken\n"
    );
    fs::remove_file(f.0.join("config.toml")).unwrap();
    assert!(f.command("uninstall").status.success());
    assert!(!f.0.join("config.toml").exists());
}

#[test]
fn uninstall_leaves_developer_executable_and_later_user_tab_setting() {
    let f = Fixture::new();
    assert!(f.command("configure").status.success());
    let text = fs::read_to_string(f.0.join("config.toml")).unwrap();
    fs::write(
        f.0.join("config.toml"),
        text.replace(
            "hide_tab_bar_when_single_tab = false",
            "hide_tab_bar_when_single_tab = true",
        ),
    )
    .unwrap();
    assert!(f.command("uninstall").status.success());
    assert!(std::path::Path::new(env!("CARGO_BIN_EXE_herdr-quota")).is_file());
    assert!(
        fs::read_to_string(f.0.join("config.toml"))
            .unwrap()
            .contains("hide_tab_bar_when_single_tab = true")
    );
}

#[test]
fn uninstall_without_install_record_can_retry_failed_reload() {
    let f = Fixture::new();
    fs::write(f.0.join("config.toml"), "[ui]\nhide_tab_bar_when_single_tab = false\ntab_bar_right = [{type='command', command='old status # herdr-quota'}]\n").unwrap();
    fs::write(f.0.join("stopped"), "").unwrap();
    assert!(!f.command("uninstall").status.success());
    assert!(!f.command("uninstall").status.success());
    fs::remove_file(f.0.join("stopped")).unwrap();
    assert!(f.command("uninstall").status.success());
    assert!(
        fs::read_to_string(f.0.join("config.toml"))
            .unwrap()
            .contains("hide_tab_bar_when_single_tab = false")
    );
}

#[test]
fn configure_and_uninstall_preserve_existing_config_permissions_under_restrictive_umask() {
    let f = Fixture::new();
    let config = f.0.join("config.toml");
    fs::write(&config, "[ui]\nhide_tab_bar_when_single_tab = true\n").unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o640)).unwrap();
    for command in ["configure", "uninstall"] {
        let output = f.command(command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::metadata(&config).unwrap().permissions().mode() & 0o777,
            0o640,
            "after {command}"
        );
    }
}

#[test]
fn configure_and_uninstall_preserve_tab_setting_inline_comment() {
    let f = Fixture::new();
    let config = f.0.join("config.toml");
    fs::write(
        &config,
        "[ui]\nhide_tab_bar_when_single_tab = true # keep my explanation\n",
    )
    .unwrap();
    assert!(f.command("configure").status.success());
    let installed = fs::read_to_string(&config).unwrap();
    assert!(installed.contains("hide_tab_bar_when_single_tab = false # keep my explanation"));
    // A later user edit to the comment must survive restoration of the old value.
    fs::write(
        &config,
        installed.replace("keep my explanation", "updated explanation"),
    )
    .unwrap();
    assert!(f.command("configure").status.success());
    assert!(f.command("uninstall").status.success());
    assert!(
        fs::read_to_string(&config)
            .unwrap()
            .contains("hide_tab_bar_when_single_tab = true # updated explanation")
    );
}

#[test]
fn corrupt_install_record_is_retained_on_uninstall_and_repaired_by_install() {
    let f = Fixture::new();
    assert!(f.run().status.success());
    let config = f.0.join("config.toml");
    let original = fs::read_to_string(&config).unwrap();
    let state = f.0.join("data/herdr-quota/install.json");
    fs::write(&state, "{broken").unwrap();
    let output = f.command("uninstall");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("repair before rerunning uninstall"));
    assert_eq!(fs::read_to_string(&state).unwrap(), "{broken");
    assert_eq!(fs::read_to_string(&config).unwrap(), original);
    assert!(f.0.join("bin/herdr-quota").exists());
    assert!(f.run().status.success());
    assert!(f.command("uninstall").status.success());
}

#[test]
fn configuring_installed_program_retains_ownership_for_uninstall() {
    let f = Fixture::new();
    let binary = f.0.join("bin/herdr-quota");
    assert!(f.run().status.success());
    let output = f.cli("configure", &binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(f.command("uninstall").status.success());
    assert!(!binary.exists());
}
