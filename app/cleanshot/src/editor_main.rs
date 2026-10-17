#![allow(dead_code)]

//! Standalone editor binary for Windows artifact testing. The tray app uses
//! the same editor window directly; this executable opens an image path into
//! that window without starting the tray, hotkey, or single-instance shell.

mod capture;
mod clipboard;
mod editor;
mod library;
mod pin;
mod platform;
mod save_dialog;
mod settings;
mod shortcut;

fn main() {
    #[cfg(windows)]
    capture::configure_dpi_awareness();

    let path = std::env::args().nth(1).filter(|arg| arg != "--help");
    let Some(path) = path else {
        eprintln!("Usage: snipsy-editor <image.png>");
        return;
    };
    let png = match std::fs::read(&path) {
        Ok(png) => png,
        Err(error) => {
            eprintln!("snipsy-editor: could not read {path}: {error}");
            return;
        }
    };
    let title = std::path::Path::new(&path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Capture".to_string());

    gpui_platform::application().run(move |cx: &mut gpui::App| {
        cx.bind_keys(editor::window::keybindings());
        if let Err(error) = editor::window::open(
            cx,
            editor::window::EditorSource {
                png,
                title,
                annotations: library::AnnotationSource::Transient,
            },
        ) {
            eprintln!("snipsy-editor: {error}");
            cx.quit();
        }
    });
}
