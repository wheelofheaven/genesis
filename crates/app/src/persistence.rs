//! Native quicksave location and input feedback. Domain snapshots live in sim_core.
use crate::session::Session;
use std::path::PathBuf;

fn path() -> genesis_storage::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or("No home directory for saves")?;
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(home).join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(home).join(".local/share"))
    };
    Ok(base.join("Genesis/world.json"))
}

pub fn quicksave(session: &mut Session) {
    let result = (|| {
        let path = path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        session.save_to(&path)
    })();
    match result {
        Ok(()) => {
            session.message = "World saved".into();
            session.message_is_error = false;
        }
        Err(e) => {
            session.message = format!("Save failed: {e}");
            session.message_is_error = true;
        }
    }
}

pub fn quickload(session: &mut Session) -> bool {
    match path().and_then(|path| session.load_from(&path)) {
        Ok(()) => true,
        Err(e) => {
            session.message = format!("Load failed: {e}");
            session.message_is_error = true;
            false
        }
    }
}
