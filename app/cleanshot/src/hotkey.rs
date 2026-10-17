//! Global hotkey, ported from `src-tauri/src/hotkeys.rs`' registration side.
//!
//! Windows: `RegisterHotKey` with a NULL window handle posts WM_HOTKEY to the
//! registering thread's message queue, so a dedicated thread runs a
//! `GetMessageW` pump and forwards events over a channel. This keeps the
//! hotkey independent of GPUI's message loop.
//!
//! Linux: no global hotkey in the dev loop — use `snipsy --capture`.
//! (The `global-hotkey` crate wants its own event-loop integration; revisit
//! if the dev loop needs hotkeys.)

use crossbeam_channel::Receiver;

#[cfg(windows)]
use crate::settings::current_settings;

pub struct HotkeyEvents {
    pub receiver: Receiver<()>,
}

#[cfg(windows)]
pub fn start() -> Result<HotkeyEvents, String> {
    let shortcut = current_settings().capture_hotkey;
    let (mods, vk) = crate::shortcut::parse_shortcut(&shortcut)
        .ok_or_else(|| format!("Could not parse the capture hotkey {shortcut:?}."))?;
    let (sender, receiver) = crossbeam_channel::bounded::<()>(0);
    std::thread::Builder::new()
        .name("snipsy-hotkey".into())
        .spawn(move || hotkey_thread::run(mods, vk, sender))
        .map_err(|error| format!("Could not start the hotkey thread: {error}"))?;
    Ok(HotkeyEvents { receiver })
}

#[cfg(not(windows))]
pub fn start() -> Result<HotkeyEvents, String> {
    let (_, receiver) = crossbeam_channel::bounded::<()>(0);
    Ok(HotkeyEvents { receiver })
}

#[cfg(windows)]
mod hotkey_thread {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, TranslateMessage, WM_HOTKEY,
    };

    const HOTKEY_ID: i32 = 1;

    pub(super) fn run(mods: u32, vk: u32, sender: crossbeam_channel::Sender<()>) {
        unsafe {
            if RegisterHotKey(std::ptr::null_mut(), HOTKEY_ID, mods, vk) == 0 {
                eprintln!(
                    "Snipsy: could not register the global hotkey (error {}); \
                     is another instance running?",
                    std::io::Error::last_os_error()
                );
                return;
            }

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                if msg.message == WM_HOTKEY {
                    // The receiver sits on the GPUI main thread's async loop;
                    // a closed channel means the app is shutting down.
                    if sender.send(()).is_err() {
                        return;
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
