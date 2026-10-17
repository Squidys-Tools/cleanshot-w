//! Global hotkey, ported from `src-tauri/src/hotkeys.rs`' registration side.
//!
//! Windows: `RegisterHotKey` with a NULL window handle posts WM_HOTKEY to the
//! registering thread's message queue, so a dedicated thread runs a
//! `GetMessageW` pump and forwards events over a channel. This keeps the
//! hotkey independent of GPUI's message loop.
//!
//! Linux: no global hotkey in the dev loop — use `cleanshot-w --capture`.
//! (The `global-hotkey` crate wants its own event-loop integration; revisit
//! if the dev loop needs hotkeys.)

use crossbeam_channel::Receiver;

#[cfg(windows)]
use crate::settings::current_settings;

pub struct HotkeyEvents {
    pub receiver: Receiver<()>,
}

const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;

#[cfg(windows)]
pub fn start() -> Result<HotkeyEvents, String> {
    let shortcut = current_settings().capture_hotkey;
    let (mods, vk) = parse_shortcut(&shortcut)
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

/// Parse a shortcut like "ctrl+shift+4" into Win32 modifier flags and a
/// virtual-key code.
pub(crate) fn parse_shortcut(shortcut: &str) -> Option<(u32, u32)> {
    let mut mods = MOD_NOREPEAT;
    let mut key = None;
    for part in shortcut.trim().split('+') {
        match part.trim() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" | "super" | "cmd" | "meta" => mods |= MOD_WIN,
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(parse_key(other)?);
            }
        }
    }
    Some((mods, key?))
}

fn parse_key(key: &str) -> Option<u32> {
    let lower = key.trim().to_ascii_lowercase();
    if lower == "space" {
        return Some(0x20);
    }
    if let Some(number) = lower.strip_prefix('f') {
        return number
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map(|n| 0x6F + n);
    }
    match lower.as_bytes() {
        [digit @ b'0'..=b'9'] => Some(u32::from(*digit)),
        [letter @ b'a'..=b'z'] => Some(u32::from(letter.to_ascii_uppercase())),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::parse_shortcut;

    #[test]
    fn parses_the_default_shortcut() {
        let (mods, vk) = parse_shortcut("ctrl+shift+4").unwrap();
        assert_eq!(vk, '4' as u32);
        assert_ne!(mods & 0xF, 0);
        assert_eq!(parse_shortcut("alt+space").unwrap().1, 0x20);
        assert_eq!(parse_shortcut("ctrl+shift+f10").unwrap().1, 0x79);
        assert_eq!(parse_shortcut("win+f24").unwrap().1, 0x87);
    }

    #[test]
    fn rejects_multi_key_and_empty_shortcuts() {
        assert!(parse_shortcut("ctrl+shift+4+5").is_none());
        assert!(parse_shortcut("ctrl").is_none());
        assert!(parse_shortcut("").is_none());
        assert!(parse_shortcut("ctrl+f25").is_none());
    }
}
