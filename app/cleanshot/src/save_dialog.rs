//! "Save PNG" through a real native file dialog (rfd) — the Tauri app's
//! anchor-download hack dies here. `save_file` blocks the calling (main)
//! thread until the user picks, which is fine for phase 1.

use std::io::Write;

/// Write `png` to the user-chosen path. Returns `Ok(false)` when the dialog
/// was cancelled.
pub fn save_png_via_dialog(png: &[u8], title: &str) -> Result<bool, String> {
    let sanitized: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{sanitized}.png"))
        .add_filter("PNG image", &["png"])
        .save_file()
    else {
        return Ok(false);
    };
    let mut file = std::fs::File::create(&path)
        .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
    file.write_all(png)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    Ok(true)
}
