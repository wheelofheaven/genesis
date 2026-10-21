//! Native quicksave location and input feedback. Domain snapshots live in sim_core.
use crate::session::Session;
use std::path::PathBuf;

pub(crate) fn path() -> genesis_storage::Result<PathBuf> {
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

/// Shared menu/keyboard writer; tests and captures supply an isolated slot.
pub(crate) fn save_at(session: &Session, path: &std::path::Path) -> genesis_storage::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    session.save_to(path)
}

pub(crate) fn load_at(
    session: &mut Session,
    camera: &mut genesis_renderer::camera::Camera,
    path: &std::path::Path,
) -> genesis_storage::Result<()> {
    session.load_from(path)?;
    crate::terrain::home_camera(&session.sim, camera);
    if !session.sim.buildings.is_empty() {
        camera.center(session.selected_point());
        camera.set_visible_height(genesis_sim_core::units::metres(100.));
    }
    Ok(())
}

pub fn quicksave(session: &mut Session) {
    let result = (|| {
        let path = path()?;
        save_at(session, &path)
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

pub fn quickload(session: &mut Session, camera: &mut genesis_renderer::camera::Camera) -> bool {
    match path().and_then(|path| load_at(session, camera, &path)) {
        Ok(()) => true,
        Err(e) => {
            session.message = format!("Load failed: {e}");
            session.message_is_error = true;
            false
        }
    }
}
