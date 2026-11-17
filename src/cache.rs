use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use crate::model::Cache;

pub fn default_dir() -> io::Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|p| Path::new(p).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| io::Error::other("cache directory unavailable"))?;
    Ok(base.join("herdr-quota"))
}

pub fn read(dir: &Path) -> Cache {
    let result = (|| {
        let file = File::open(dir.join("quota.json"))?;
        if file.metadata()?.len() > 65_536 {
            return Err(io::Error::other("invalid cache"));
        }
        let cache: Cache = serde_json::from_reader(file.take(65_537))?;
        if cache.quota.as_ref().is_some_and(|q| {
            [&q.five_hour, &q.weekly]
                .iter()
                .any(|w| w.remaining.is_some_and(|p| p > 100))
        }) {
            return Err(io::Error::other("invalid cache"));
        }
        Ok(cache)
    })();
    result.unwrap_or_default()
}

pub fn lock(dir: &Path) -> io::Result<Option<File>> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(dir.join("refresh.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

pub fn write(dir: &Path, cache: &Cache) -> io::Result<()> {
    let temporary = dir.join(format!("quota.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        serde_json::to_writer(&mut file, cache)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temporary, dir.join("quota.json"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
