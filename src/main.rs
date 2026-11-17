mod cache;
mod codex;
mod http_quota;
mod management;
mod model;

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use codex::{Account, Codex};
use model::{Cache, State};

struct Options {
    command: String,
    cache_dir: PathBuf,
    codex: PathBuf,
    herdr: PathBuf,
    config: Option<PathBuf>,
    install_dir: Option<PathBuf>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn options() -> Result<Options, &'static str> {
    let mut args = std::env::args_os().skip(1);
    let command = args
        .next()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "status".into());
    let mut options = Options {
        command,
        cache_dir: cache::default_dir().map_err(|_| "cache directory unavailable")?,
        codex: "codex".into(),
        herdr: "herdr".into(),
        config: None,
        install_dir: None,
    };
    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--cache-dir") => {
                options.cache_dir = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("missing cache directory")?
            }
            Some("--codex") => {
                options.codex = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("missing Codex executable")?
            }
            Some("--herdr") => {
                options.herdr = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("missing Herdr executable")?
            }
            Some("--config") => {
                options.config = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or("missing config path")?,
                )
            }
            Some("--install-dir") => {
                options.install_dir = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or("missing install directory")?,
                )
            }
            _ => return Err("unknown option"),
        }
    }
    if !options.cache_dir.is_absolute() {
        return Err("cache directory must be absolute");
    }
    if options.codex.components().count() > 1 && !options.codex.is_absolute() {
        return Err("Codex executable must be a command name or absolute path");
    }
    Ok(options)
}

fn refresh(options: &Options) -> std::io::Result<()> {
    let Some(_lock) = cache::lock(&options.cache_dir)? else {
        return Ok(());
    };
    let previous = cache::read(&options.cache_dir);
    let started = now();
    if !previous.due(started) {
        return Ok(());
    }
    let mut current = previous.clone();
    current.attempted_at = started;
    // Publish the attempt before any network access, to rate-limit failures.
    cache::write(&options.cache_dir, &current)?;
    if let Ok((identity, quota)) = http_quota::read(&options.codex) {
        return cache::write(
            &options.cache_dir,
            &Cache {
                attempted_at: started,
                fetched_at: Some(now()),
                identity: Some(identity),
                quota: Some(quota),
                state: State::Ready,
            },
        );
    }
    // Codex owns refresh-token recovery and unsupported credential stores.
    let Ok(mut client) = Codex::start(&options.codex) else {
        // Without a verified account, retaining old quota risks misattribution.
        return cache::write(
            &options.cache_dir,
            &Cache {
                attempted_at: started,
                state: State::AccountUnknown,
                ..Cache::default()
            },
        );
    };
    match client.account() {
        Ok(Account::Chatgpt(identity)) => {
            if current.identity.as_deref() != Some(&identity) {
                current = Cache {
                    attempted_at: started,
                    identity: Some(identity),
                    ..Cache::default()
                };
                cache::write(&options.cache_dir, &current)?;
            }
            match client.quota() {
                Ok(quota) => {
                    current.quota = Some(quota);
                    current.fetched_at = Some(now());
                    current.state = State::Ready;
                }
                Err(()) => current.state = State::RefreshFailed,
            }
        }
        account => {
            let state = match account {
                Ok(Account::LoggedOut) => State::LoggedOut,
                Ok(Account::Unsupported) => State::Unsupported,
                _ => State::AccountUnknown,
            };
            current = Cache {
                attempted_at: started,
                state,
                ..Cache::default()
            };
        }
    }
    cache::write(&options.cache_dir, &current)
}

fn status(options: &Options) {
    let current = cache::read(&options.cache_dir);
    if current.due(now())
        && let Ok(executable) = std::env::current_exe()
    {
        // Herdr cleans up the status command's process group on success too.
        // Give the short-lived refresh its own group so it can finish reading.
        let _ = Command::new(executable)
            .process_group(0)
            .arg("refresh")
            .arg("--cache-dir")
            .arg(&options.cache_dir)
            .arg("--codex")
            .arg(&options.codex)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    println!("{}", current.render(now()));
}

fn herdr_config(options: &Options) -> std::io::Result<()> {
    fn quote(value: &std::ffi::OsStr) -> std::io::Result<String> {
        let text = value
            .to_str()
            .ok_or_else(|| std::io::Error::other("path must be UTF-8"))?;
        Ok(format!("'{}'", text.replace('\'', "'\\''")))
    }
    let command = format!(
        "{} status --cache-dir {} --codex {}",
        quote(std::env::current_exe()?.as_os_str())?,
        quote(options.cache_dir.as_os_str())?,
        quote(options.codex.as_os_str())?
    );
    println!(
        "[ui]\nhide_tab_bar_when_single_tab = false\ntab_bar_position = \"top\"\ntab_bar_right = [\n  {{ type = \"command\", command = {}, interval_seconds = 30, timeout_seconds = 2 }},\n]",
        serde_json::to_string(&command)?
    );
    Ok(())
}

fn main() {
    let options = match options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("herdr-quota: {message}");
            std::process::exit(2);
        }
    };
    match options.command.as_str() {
        "install" | "configure" => {
            if let Err(error) = management::install(&options, options.command == "install") {
                eprintln!("herdr-quota: {error}");
                std::process::exit(1);
            }
        }
        "uninstall" => {
            if let Err(error) = management::uninstall(&options) {
                eprintln!("herdr-quota: {error}");
                std::process::exit(1);
            }
        }
        "status" => status(&options),
        "herdr-config" => {
            if herdr_config(&options).is_err() {
                eprintln!("herdr-quota: cannot generate configuration");
                std::process::exit(1);
            }
        }
        "refresh" => {
            if refresh(&options).is_err() {
                eprintln!("herdr-quota: cannot update private quota cache");
                std::process::exit(1);
            }
            println!("{}", cache::read(&options.cache_dir).render(now()));
        }
        "--help" | "-h" | "help" => println!(
            "herdr-quota [status|refresh|herdr-config|install|configure|uninstall] [--cache-dir ABSOLUTE_PATH] [--codex EXECUTABLE]\n\ninstall       Install to ~/.local/bin, merge Herdr config and reload.\nconfigure     Merge Herdr config and reload using this executable.\nuninstall     Remove managed Herdr status, reload, then remove installed program.\nManagement options: --config ABSOLUTE_PATH --install-dir ABSOLUTE_PATH --herdr EXECUTABLE\n\nstatus        Print cached quota immediately; refresh in the background if due.\nrefresh       Perform a due refresh (HTTP 10s; app-server fallback 30s).\nherdr-config  Print a Herdr [ui] configuration snippet; does not change files."
        ),
        "--version" | "-V" => println!("herdr-quota {}", env!("CARGO_PKG_VERSION")),
        _ => {
            eprintln!("herdr-quota: unknown command; use --help");
            std::process::exit(2);
        }
    }
}
