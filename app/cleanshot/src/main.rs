//! Snipsy — pure-native GPUI Windows app.
//!
//! Boots to tray, registers the global capture hotkey, and runs the core
//! capture loop end to end: full-screen GDI snapshot → rubber-band overlay →
//! region crop → library → editor window, with clipboard copy and save-dialog.

mod capture;
mod clipboard;
mod editor;
mod flow;
mod hotkey;
mod library;
mod overlay;
mod pin;
mod platform;
mod quick_access;
mod save_dialog;
mod settings;
mod shortcut;
mod theme;
#[cfg(windows)]
mod tray;
mod ui;
mod window_picker;

fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("snipsy=info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();

    capture::configure_dpi_awareness();

    #[cfg(windows)]
    if !single_instance::acquire() {
        eprintln!("Snipsy: another instance is already running");
        return;
    }

    if let Err(error) = settings::sync_autostart() {
        eprintln!("Snipsy: could not sync launch-at-startup setting: {error}");
    }

    let args: Vec<String> = std::env::args().collect();
    let flag_capture = args.iter().any(|arg| arg == "--capture");
    let flag_pin = args
        .iter()
        .position(|arg| arg == "--pin")
        .and_then(|index| args.get(index + 1))
        .cloned();

    // The tray icon is removed when dropped, so hold it for the app's
    // lifetime. Its hidden window is pumped by GPUI's message loop.
    #[cfg(windows)]
    let _tray = match tray::create() {
        Ok(tray) => Some(tray),
        Err(error) => {
            eprintln!("Snipsy: {error}");
            None
        }
    };

    gpui_platform::application().run(move |cx: &mut gpui::App| {
        cx.set_global(flow::AppState::default());
        crate::theme::init_from_settings(cx);
        cx.bind_keys(
            overlay::keybindings()
                .into_iter()
                .chain(pin::keybindings())
                .chain(window_picker::keybindings())
                .chain(editor::window::keybindings())
                .chain(quick_access::keybindings()),
        );

        // Tray menu → app actions.
        #[cfg(windows)]
        cx.spawn(async move |cx| {
            loop {
                let Ok(event) = cx
                    .background_executor()
                    .spawn(async { tray::event_receiver().recv() })
                    .await
                else {
                    return;
                };
                cx.update(|cx| flow::handle_menu_event(cx, &event.id.0));
            }
        })
        .detach();

        // Global hotkey → capture flow.
        match hotkey::start() {
            Ok(hotkey::HotkeyEvents { receiver }) => {
                cx.spawn(async move |cx| {
                    loop {
                        let event = cx.background_executor().spawn({
                            let receiver = receiver.clone();
                            async move { receiver.recv() }
                        });
                        match event.await {
                            Ok(()) => cx.update(flow::start_capture),
                            Err(_) => return,
                        }
                    }
                })
                .detach();
            }
            Err(error) => eprintln!("Snipsy: {error}"),
        }

        // Linux dev-loop triggers (no tray, no hotkeys there yet).
        if flag_capture {
            flow::start_capture(cx);
        }
        if let Some(path) = flag_pin {
            flow::open_pin_from_path(cx, &path);
        }
    });
}

/// Single instance via a named Windows mutex: a second launch exits quietly.
#[cfg(windows)]
mod single_instance {
    use std::sync::OnceLock;

    static MUTEX_HANDLE: OnceLock<usize> = OnceLock::new();

    pub fn acquire() -> bool {
        use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let name: Vec<u16> = "Local\\Snipsy.SingleInstance\0".encode_utf16().collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            // Could not even create the mutex; let the app run rather than
            // die on a missing single-instance guard.
            eprintln!(
                "Snipsy: could not create the single-instance mutex (error {})",
                std::io::Error::last_os_error()
            );
            return true;
        }
        // The handle must stay held for the process lifetime or the mutex is
        // released; keep it (stored once, effectively leaked).
        let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let _ = MUTEX_HANDLE.set(handle as usize);
        !already_running
    }
}
