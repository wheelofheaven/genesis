//! Native host storage shared by the game and CPU runner. No simulation rules.
use genesis_sim_core::save::{Restored, Snapshot};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub const MAX_SAVE_BYTES: u64 = 128 * 1024 * 1024;

/// Random host metadata; never used as an input to domain rules or generation.
pub fn new_world_id() -> String {
    format!("{:032x}", rand::random::<u128>())
}

pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>> {
    snapshot.validate()?;
    let bytes = serde_json::to_vec(snapshot)?;
    if bytes.len() as u64 > MAX_SAVE_BYTES {
        return Err("save exceeds 128 MiB limit".into());
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Restored> {
    if bytes.len() as u64 > MAX_SAVE_BYTES {
        return Err("save exceeds 128 MiB limit".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(bytes)?;
    Ok(snapshot.restore()?)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_SAVE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SAVE_BYTES {
        return Err("save exceeds 128 MiB limit".into());
    }
    Ok(bytes)
}

pub fn load(path: &Path) -> Result<Restored> {
    decode(&read_bytes(path)?)
}

/// Sibling backup, e.g. `world.json.bak`. It is also a normal loadable save.
pub fn backup_path(path: &Path) -> Result<PathBuf> {
    let mut name = path
        .file_name()
        .ok_or("save path needs a filename")?
        .to_os_string();
    name.push(".bak");
    Ok(path.with_file_name(name))
}

/// Write completely and sync before replacing a native file. An existing valid
/// save is first backed up with its own atomic replacement. Failures before the
/// final rename leave the active file unchanged. No power-loss guarantee beyond
/// the host filesystem's rename/sync contract is claimed.
pub fn store(path: &Path, snapshot: &Snapshot) -> Result<()> {
    let bytes = encode(snapshot)?;
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err("save destination is not a regular file".into());
            }
            let old = read_bytes(path)?;
            decode(&old)
                .map_err(|e| format!("existing save is invalid; use another filename: {e}"))?;
            replace(&backup_path(path)?, &old)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    replace(path, &bytes)
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path.file_name().ok_or("save path needs a filename")?;
    let mut temporary_name = std::ffi::OsString::from(".");
    temporary_name.push(name);
    temporary_name.push(format!(".{}.tmp", new_world_id()));
    let temp_path = path.with_file_name(temporary_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;
    let temporary = Temporary(temp_path);
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary.0, path)?;
    Ok(())
}

#[cfg(test)]
mod tests;
