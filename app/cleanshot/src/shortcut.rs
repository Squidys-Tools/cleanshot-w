const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;

pub(crate) fn parse_shortcut(shortcut: &str) -> Option<(u32, u32)> {
    let mut modifiers = MOD_NOREPEAT;
    let mut key = None;
    for part in shortcut.trim().split('+') {
        match part.trim() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "shift" => modifiers |= MOD_SHIFT,
            "alt" => modifiers |= MOD_ALT,
            "win" | "super" | "cmd" | "meta" => modifiers |= MOD_WIN,
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(parse_key(other)?);
            }
        }
    }
    Some((modifiers, key?))
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
            .filter(|number| (1..=24).contains(number))
            .map(|number| 0x6F + number);
    }
    match lower.as_bytes() {
        [digit @ b'0'..=b'9'] => Some(u32::from(*digit)),
        [letter @ b'a'..=b'z'] => Some(u32::from(letter.to_ascii_uppercase())),
        _ => None,
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
