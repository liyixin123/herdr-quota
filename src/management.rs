//! Installation boundary: filesystem configuration and external Herdr/Codex commands.
use crate::{
    Options,
    codex::{Account, Codex},
};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use toml_edit::{DocumentMut, InlineTable, Item, Value, value};

/// Both full installation records and interrupted-uninstall recovery records use this shape.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct InstallationRecord {
    #[serde(default)]
    binary: Option<PathBuf>,
    #[serde(default)]
    config: Option<PathBuf>,
    // Missing ownership must never authorize deleting a developer executable.
    #[serde(default)]
    installed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "PreviousHide::is_unrecorded")]
    previous_hide: PreviousHide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ui_existed: Option<bool>,
}

// A missing field in a recovery record means no restoration is authorized;
// explicit null in an installation record means the original setting was absent.
#[derive(Default)]
enum PreviousHide {
    #[default]
    Unrecorded,
    Recorded(Option<String>),
}
impl PreviousHide {
    fn is_unrecorded(&self) -> bool {
        matches!(self, Self::Unrecorded)
    }
}
impl serde::Serialize for PreviousHide {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unrecorded => None::<String>.serialize(serializer),
            Self::Recorded(value) => value.serialize(serializer),
        }
    }
}
impl<'de> serde::Deserialize<'de> for PreviousHide {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<String>::deserialize(deserializer).map(Self::Recorded)
    }
}
impl InstallationRecord {
    fn read(path: &Path) -> io::Result<Option<Self>> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| {
                io::Error::other(format!("cannot parse installation state {}; retain it for recovery and repair before rerunning uninstall: {e}", path.display()))
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    fn write(&self, path: &Path) -> io::Result<()> {
        atomic_write(path, serde_json::to_string_pretty(self)?.as_bytes(), 0o600)
    }
}

pub fn home() -> io::Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| io::Error::other("HOME must be an absolute path"))
}
pub fn config_path() -> io::Result<PathBuf> {
    if let Some(p) = env::var_os("HERDR_CONFIG_PATH") {
        return Ok(p.into());
    }
    Ok(env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".config"))
        .join("herdr/config.toml"))
}
pub fn state_path() -> io::Result<PathBuf> {
    Ok(env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".local/share"))
        .join("herdr-quota/install.json"))
}
pub fn quote(path: &Path) -> io::Result<String> {
    Ok(format!(
        "'{}'",
        path.to_str()
            .ok_or_else(|| io::Error::other("path must be UTF-8"))?
            .replace('\'', "'\\''")
    ))
}
pub fn resolve(path: &Path, repair: &str) -> io::Result<PathBuf> {
    let candidates = if path.components().count() > 1 {
        vec![path.to_owned()]
    } else {
        env::split_paths(&env::var_os("PATH").unwrap_or_default())
            .map(|p| p.join(path))
            .collect()
    };
    candidates
        .into_iter()
        .find(|p| fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
        .map(fs::canonicalize)
        .transpose()?
        .ok_or_else(|| io::Error::other(repair.to_owned()))
}
pub fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    use std::io::Write;
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| io::Error::other("path needs parent"))?,
    )?;
    let temp = path.with_file_name(format!(".herdr-quota-{}-tmp", std::process::id()));
    let result = (|| {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn managed(command: &str) -> bool {
    command.ends_with(" # herdr-quota")
}

/// Remove only marked commands, for either supported TOML representation.
fn remove_managed_entries(entries: &mut Item) -> io::Result<()> {
    if let Some(array) = entries.as_array_mut() {
        array.retain(|entry| {
            !entry
                .as_inline_table()
                .and_then(|t| t.get("command"))
                .and_then(Value::as_str)
                .is_some_and(managed)
        });
    } else if let Some(array) = entries.as_array_of_tables_mut() {
        array.retain(|entry| {
            !entry
                .get("command")
                .and_then(Item::as_str)
                .is_some_and(managed)
        });
    } else {
        return Err(io::Error::other(
            "ui.tab_bar_right must be an array; repair config and rerun",
        ));
    }
    Ok(())
}

/// Replace a value while retaining the user's surrounding whitespace and comment.
fn replace_value(ui: &mut dyn toml_edit::TableLike, key: &str, mut replacement: Item) {
    if let Some(current) = ui.get(key).and_then(Item::as_value)
        && let Some(value) = replacement.as_value_mut()
    {
        *value.decor_mut() = current.decor().clone();
    }
    ui.insert(key, replacement);
}

pub fn install(options: &Options, copy: bool) -> io::Result<()> {
    let config = options.config.clone().unwrap_or(config_path()?);
    let dir = options
        .install_dir
        .clone()
        .unwrap_or(home()?.join(".local/bin"));
    let state = state_path()?;
    for p in [&config, &dir, &state] {
        if !p.is_absolute() {
            return Err(io::Error::other(
                "configuration, data and install paths must be absolute",
            ));
        }
    }
    let codex = resolve(
        &options.codex,
        "Codex is missing; install Codex CLI, run codex login, then rerun this command",
    )?;
    let herdr = resolve(
        &options.herdr,
        "Herdr is missing; install Herdr, start it, then rerun this command",
    )?;
    let mut client = Codex::start(&codex)
        .map_err(|_| io::Error::other("cannot connect to Codex; run codex login and rerun"))?;
    if !matches!(client.account(), Ok(Account::Chatgpt(_))) {
        return Err(io::Error::other(
            "Codex needs a ChatGPT subscription login; run codex login, then rerun",
        ));
    }
    drop(client);
    let source = env::current_exe()?;
    let binary = if copy {
        dir.join("herdr-quota")
    } else {
        source.clone()
    };
    let original = match fs::read_to_string(&config) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut doc = original.parse::<DocumentMut>().map_err(|e| {
        io::Error::other(format!(
            "cannot parse {}; repair TOML and rerun: {e}",
            config.display()
        ))
    })?;
    let ui_existed = doc.get("ui").is_some();
    if !ui_existed {
        doc["ui"] = Item::Table(toml_edit::Table::new());
    }
    let ui = doc["ui"]
        .as_table_like_mut()
        .ok_or_else(|| io::Error::other("ui must be a TOML table; repair config and rerun"))?;
    let previous_hide = ui
        .get("hide_tab_bar_when_single_tab")
        .map(ToString::to_string);
    if ui.get("tab_bar_right").is_none() {
        ui.insert("tab_bar_right", value(toml_edit::Array::new()));
    }
    let command = format!(
        "{} status --cache-dir {} --codex {} # herdr-quota",
        quote(&binary)?,
        quote(&options.cache_dir)?,
        quote(&codex)?
    );
    let entries = ui.get_mut("tab_bar_right").expect("inserted status array");
    remove_managed_entries(entries)?;
    if let Some(entries) = entries.as_array_mut() {
        let mut entry = InlineTable::new();
        entry.insert("type", Value::from("command"));
        entry.insert("command", Value::from(command.as_str()));
        entry.insert("interval_seconds", Value::from(5));
        entry.insert("timeout_seconds", Value::from(2));
        entries.push(entry);
    } else if let Some(entries) = entries.as_array_of_tables_mut() {
        let mut entry = toml_edit::Table::new();
        entry.insert("type", value("command"));
        entry.insert("command", value(command.as_str()));
        entry.insert("interval_seconds", value(5));
        entry.insert("timeout_seconds", value(2));
        entries.push(entry);
    } else {
        return Err(io::Error::other(
            "ui.tab_bar_right must be an array; repair config and rerun",
        ));
    }
    replace_value(ui, "hide_tab_bar_when_single_tab", value(false));
    let updated = doc.to_string();
    // Persist ownership before mutation, allowing retry after an interrupted write/reload.
    // Installation can repair an absent/corrupt record; uninstall retains a corrupt record.
    let old_state = InstallationRecord::read(&state)
        .ok()
        .flatten()
        .unwrap_or_default();
    let same_config = old_state.config.as_ref() == Some(&config);
    let saved = InstallationRecord {
        binary: Some(binary.clone()),
        installed: copy
            || (same_config && old_state.installed && old_state.binary.as_ref() == Some(&binary)),
        config: Some(config.clone()),
        command: Some(command.clone()),
        previous_hide: if same_config {
            old_state.previous_hide
        } else {
            PreviousHide::Recorded(previous_hide)
        },
        ui_existed: if same_config {
            old_state.ui_existed
        } else {
            Some(ui_existed)
        },
    };
    saved.write(&state)?;
    if copy && source != binary {
        atomic_write(&binary, &fs::read(source)?, 0o755)?;
    }
    println!("Program: {}", binary.display());
    if updated != original {
        if config.exists() {
            let mut n = 0;
            let backup = loop {
                let p = config.with_file_name(format!(
                    "config.toml.herdr-quota-backup-{}-{n}",
                    std::process::id()
                ));
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&p)
                {
                    Ok(mut f) => {
                        use std::io::Write;
                        f.write_all(original.as_bytes())?;
                        break p;
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
                    Err(e) => return Err(e),
                }
            };
            println!("Backup: {}", backup.display());
        }
        let mode = fs::metadata(&config)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0o600);
        atomic_write(&config, updated.as_bytes(), mode).map_err(|e| {
            io::Error::other(format!(
                "cannot write {}; original retained; fix permissions and rerun: {e}",
                config.display()
            ))
        })?;
    }
    println!(
        "Config: {}\nui.hide_tab_bar_when_single_tab = false\nui.tab_bar_right += {command}",
        config.display()
    );
    let result = Command::new(herdr)
        .args(["server", "reload-config"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !result.success() {
        return Err(io::Error::other(
            "Herdr reload failed; start the local Herdr server, verify its config path, then rerun the same command",
        ));
    }
    println!("Herdr configuration reloaded. Installation complete.");
    Ok(())
}

/// Remove only the owned status entry; never restore a whole installation backup.
pub fn uninstall(options: &Options) -> io::Result<()> {
    let state = state_path()?;
    let mut saved = InstallationRecord::read(&state)?;
    let recorded_config = saved.as_ref().and_then(|s| s.config.clone());
    if let (Some(requested), Some(recorded)) = (&options.config, &recorded_config)
        && requested != recorded
    {
        return Err(io::Error::other(
            "requested config differs from installation state; rerun uninstall with the recorded config",
        ));
    }
    let config = options
        .config
        .clone()
        .or(recorded_config)
        .unwrap_or(config_path()?);
    let binary = saved.as_ref().and_then(|s| s.binary.clone()).unwrap_or(
        options
            .install_dir
            .clone()
            .unwrap_or(home()?.join(".local/bin"))
            .join("herdr-quota"),
    );
    if !config.is_absolute() || !binary.is_absolute() || !state.is_absolute() {
        return Err(io::Error::other(
            "configuration, data and binary paths must be absolute",
        ));
    }
    let original = match fs::read_to_string(&config) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(io::Error::other(format!(
                "cannot read {}; binary and recovery state retained; repair and rerun uninstall: {e}",
                config.display()
            )));
        }
    };
    let mut doc = original.parse::<DocumentMut>().map_err(|e| {
        io::Error::other(format!(
            "cannot parse {}; repair TOML and rerun uninstall; binary and state retained: {e}",
            config.display()
        ))
    })?;
    if let Some(ui) = doc.get_mut("ui") {
        let ui = ui.as_table_like_mut().ok_or_else(|| {
            io::Error::other("ui must be a TOML table; repair and rerun uninstall")
        })?;
        if let Some(entries) = ui.get_mut("tab_bar_right") {
            remove_managed_entries(entries)?;
        }
        // Restore the setting only while it still has our installed value.
        if ui
            .get("hide_tab_bar_when_single_tab")
            .and_then(Item::as_bool)
            == Some(false)
            && let Some(saved) = &saved
        {
            if let PreviousHide::Recorded(Some(previous)) = &saved.previous_hide {
                let restored = format!("value = {}", previous.trim())
                    .parse::<DocumentMut>()
                    .map_err(|e| {
                        io::Error::other(format!(
                            "invalid previous tab setting in installation state: {e}"
                        ))
                    })?;
                replace_value(
                    ui,
                    "hide_tab_bar_when_single_tab",
                    restored["value"].clone(),
                );
            } else if matches!(saved.previous_hide, PreviousHide::Recorded(None)) {
                ui.remove("hide_tab_bar_when_single_tab");
            }
        }
    }
    let updated = doc.to_string();
    // A missing install record must not lose a pending reload after config cleanup.
    if saved.is_none() && (updated != original || binary.exists()) {
        let recovery = InstallationRecord {
            binary: Some(binary.clone()),
            config: Some(config.clone()),
            installed: true,
            ..InstallationRecord::default()
        };
        recovery.write(&state)?;
        saved = Some(recovery);
    }
    if updated != original {
        let mut n = 0;
        let backup = loop {
            let backup = config.with_file_name(format!(
                "config.toml.herdr-quota-uninstall-backup-{}-{n}",
                std::process::id()
            ));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&backup)
            {
                Ok(mut file) => {
                    use std::io::Write;
                    file.write_all(original.as_bytes())?;
                    break backup;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
                Err(e) => {
                    return Err(io::Error::other(format!(
                        "cannot back up config; nothing removed; fix permissions and rerun uninstall: {e}"
                    )));
                }
            }
        };
        println!("Backup: {}", backup.display());
        let mode = fs::metadata(&config)?.permissions().mode() & 0o777;
        atomic_write(&config, updated.as_bytes(), mode).map_err(|e| io::Error::other(format!("cannot write {}; binary and recovery state retained; fix permissions and rerun uninstall: {e}", config.display())))?;
    }
    println!(
        "Config: {}\nManaged quota status removed (or already absent).",
        config.display()
    );
    if saved.is_some() || updated != original || binary.exists() {
        let herdr = resolve(
            &options.herdr,
            "Herdr executable missing; config cleaned, binary and recovery state retained; install Herdr or specify --herdr and rerun uninstall",
        )?;
        let result = Command::new(&herdr)
            .args(["server", "reload-config"])
            .stdin(Stdio::null())
            .output()?;
        if !result.status.success() {
            let status = Command::new(&herdr)
                .args(["status", "--json"])
                .stdin(Stdio::null())
                .output()?;
            let stopped = status.status.success()
                && serde_json::from_slice::<serde_json::Value>(&status.stdout).is_ok_and(|s| {
                    s["server"]["status"] == "not_running" && s["server"]["running"] == false
                });
            if !stopped {
                return Err(io::Error::other(
                    "Herdr reload failed; config cleaned, binary and recovery state retained; repair Herdr configuration or start the local server and rerun uninstall",
                ));
            }
            println!("Herdr is not running; configuration cleanup takes effect on its next start.");
        } else {
            println!("Herdr configuration reloaded.");
        }
    }
    // configure uses the developer executable directly, so it must never delete it.
    if saved.as_ref().is_none_or(|s| s.installed) {
        match fs::remove_file(&binary) {
            Ok(()) => println!("Removed program: {}", binary.display()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                println!("Program already absent: {}", binary.display())
            }
            Err(e) => {
                return Err(io::Error::other(format!(
                    "config cleaned; cannot remove {}; recovery state retained; fix permissions and rerun uninstall: {e}",
                    binary.display()
                )));
            }
        }
    }
    match fs::remove_file(&state) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => {
            return Err(io::Error::other(format!(
                "program removed; cannot remove recovery state; rerun uninstall: {e}"
            )));
        }
    }
    println!("Uninstallation complete.");
    Ok(())
}
