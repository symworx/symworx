use crossterm::event::KeyCode;

use crate::app::App;

pub(crate) fn arm_pending_delete(app: &mut App) {
    match app.selected_path().cloned() {
        Some(path) if path.is_file() => {
            app.pending_delete = Some(path.clone());
            app.clear_esc_quit();
            app.status = format!("Delete {} ?   y confirm   n / Esc cancel", path.display());
        }
        Some(path) => {
            app.status = format!("Not a file: {}", path.display());
        }
        None => {
            app.status = "Select a file first, then x to delete".into();
        }
    }
}

/// Handle keys while `pending_delete` is armed. Returns true if the modal consumed the key.
pub(crate) fn handle_pending_delete_keys(app: &mut App, code: KeyCode) -> bool {
    if app.pending_delete.is_none() {
        return false;
    }
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            confirm_pending_delete(app);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_delete = None;
            app.clear_esc_quit();
            app.status = "Delete cancelled".into();
        }
        // Swallow other keys so they don't leak into path/filter while confirming.
        _ => {}
    }
    true
}

pub(crate) fn confirm_pending_delete(app: &mut App) {
    let Some(path) = app.pending_delete.take() else {
        return;
    };
    let name = path.display().to_string();
    match std::fs::remove_file(&path) {
        Ok(()) => {
            // Best-effort: remove peaks sidecar next to biosignal CSVs.
            let peaks = crate::generate::peaks_sidecar_path(&path);
            if peaks.is_file() {
                let _ = std::fs::remove_file(&peaks);
            }
            app.refresh_file_list();
            app.ensure_valid_selection();
            app.status = format!("Deleted {name}");
        }
        Err(e) => {
            app.status = format!("Delete failed ({name}): {e}");
        }
    }
}
