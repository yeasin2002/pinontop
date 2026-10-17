use thiserror::Error;

// Win32 Virtual Key Code definitions
pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_PRIOR: u32 = 0x21; // Page Up
pub const VK_NEXT: u32 = 0x22;  // Page Down
pub const VK_END: u32 = 0x23;
pub const VK_HOME: u32 = 0x24;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_INSERT: u32 = 0x2D;
pub const VK_DELETE: u32 = 0x2E;
pub const VK_NUMPAD0: u32 = 0x60;
pub const VK_NUMPAD9: u32 = 0x69;
pub const VK_MULTIPLY: u32 = 0x6A;
pub const VK_ADD: u32 = 0x6B;
pub const VK_SUBTRACT: u32 = 0x6D;
pub const VK_DIVIDE: u32 = 0x6F;
pub const VK_F1: u32 = 0x70;
pub const VK_F9: u32 = 0x78;
pub const VK_F24: u32 = 0x87;
pub const VK_OEM_1: u32 = 0xBA;      // Semicolon
pub const VK_OEM_PLUS: u32 = 0xBB;   // =
pub const VK_OEM_COMMA: u32 = 0xBC;  // ,
pub const VK_OEM_MINUS: u32 = 0xBD;  // -
pub const VK_OEM_PERIOD: u32 = 0xBE; // .
pub const VK_OEM_2: u32 = 0xBF;      // /
pub const VK_OEM_3: u32 = 0xC0;      // `
pub const VK_OEM_4: u32 = 0xDB;      // [
pub const VK_OEM_5: u32 = 0xDC;      // \
pub const VK_OEM_6: u32 = 0xDD;      // ]
pub const VK_OEM_7: u32 = 0xDE;      // '

// Win32 Hotkey Modifiers
pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

// Stable Hotkey IDs
pub const HOTKEY_PIN: i32 = 1;
pub const HOTKEY_OPACITY_UP: i32 = 2;
pub const HOTKEY_OPACITY_DOWN: i32 = 3;
pub const HOTKEY_WINDOW: i32 = 4;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ShortcutError {
    #[error("Shortcut string cannot be empty")]
    Empty,
    #[error("Shortcut must include at least one safe modifier: Win, Ctrl, or Alt")]
    UnsafeModifier,
    #[error("Unrecognized key token: {0}")]
    UnknownKey(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedShortcut {
    pub modifiers: u32,
    pub vk: u32,
    pub key_token: String,
    pub canonical_string: String,
}

/// Enforces the Safe Modifier Rule:
/// Shortcuts must include at least one of Win, Ctrl, or Alt to avoid capturing standard typing.
pub fn is_safe_modifier(modifiers: u32) -> bool {
    (modifiers & (MOD_WIN | MOD_CONTROL | MOD_ALT)) != 0
}

/// Converts a Web API `KeyboardEvent.code` token to a Win32 Virtual Key code.
pub fn key_token_to_vk(token: &str) -> Option<u32> {
    // Letters KeyA - KeyZ
    if token.starts_with("Key") && token.len() == 4 {
        let ch = token.chars().nth(3)?;
        if ch.is_ascii_alphabetic() {
            return Some(ch.to_ascii_uppercase() as u32);
        }
    }

    // Number row Digit0 - Digit9
    if token.starts_with("Digit") && token.len() == 6 {
        let ch = token.chars().nth(5)?;
        if ch.is_ascii_digit() {
            return Some(ch as u32);
        }
    }

    // Function keys F1 - F24
    if token.starts_with('F')
        && let Ok(num) = token[1..].parse::<u32>()
            && (1..=24).contains(&num) {
                return Some(VK_F1 + (num - 1));
            }

    // Numpad 0-9
    if token.starts_with("Numpad") && token.len() == 7
        && let Ok(num) = token[6..].parse::<u32>()
            && num <= 9 {
                return Some(VK_NUMPAD0 + num);
            }

    match token {
        "Equal" => Some(VK_OEM_PLUS),
        "Minus" => Some(VK_OEM_MINUS),
        "Space" => Some(VK_SPACE),
        "Enter" => Some(VK_RETURN),
        "Escape" => Some(VK_ESCAPE),
        "Tab" => Some(VK_TAB),
        "Backspace" => Some(VK_BACK),
        "Delete" => Some(VK_DELETE),
        "Insert" => Some(VK_INSERT),
        "PageUp" => Some(VK_PRIOR),
        "PageDown" => Some(VK_NEXT),
        "Home" => Some(VK_HOME),
        "End" => Some(VK_END),
        "ArrowUp" => Some(VK_UP),
        "ArrowDown" => Some(VK_DOWN),
        "ArrowLeft" => Some(VK_LEFT),
        "ArrowRight" => Some(VK_RIGHT),
        "NumpadAdd" => Some(VK_ADD),
        "NumpadSubtract" => Some(VK_SUBTRACT),
        "NumpadMultiply" => Some(VK_MULTIPLY),
        "NumpadDivide" => Some(VK_DIVIDE),
        "BracketLeft" => Some(VK_OEM_4),
        "BracketRight" => Some(VK_OEM_6),
        "Semicolon" => Some(VK_OEM_1),
        "Quote" => Some(VK_OEM_7),
        "Backquote" => Some(VK_OEM_3),
        "Backslash" => Some(VK_OEM_5),
        "Comma" => Some(VK_OEM_COMMA),
        "Period" => Some(VK_OEM_PERIOD),
        "Slash" => Some(VK_OEM_2),
        _ => None,
    }
}

/// Converts a Win32 Virtual Key code back to its Web API key token.
pub fn vk_to_key_token(vk: u32) -> String {
    match vk {
        0x41..=0x5A => format!("Key{}", (vk as u8 as char)),
        0x30..=0x39 => format!("Digit{}", (vk as u8 as char)),
        VK_F1..=VK_F24 => format!("F{}", vk - VK_F1 + 1),
        VK_NUMPAD0..=VK_NUMPAD9 => format!("Numpad{}", vk - VK_NUMPAD0),
        VK_OEM_PLUS => "Equal".to_string(),
        VK_OEM_MINUS => "Minus".to_string(),
        VK_SPACE => "Space".to_string(),
        VK_RETURN => "Enter".to_string(),
        VK_ESCAPE => "Escape".to_string(),
        VK_TAB => "Tab".to_string(),
        VK_BACK => "Backspace".to_string(),
        VK_DELETE => "Delete".to_string(),
        VK_INSERT => "Insert".to_string(),
        VK_PRIOR => "PageUp".to_string(),
        VK_NEXT => "PageDown".to_string(),
        VK_HOME => "Home".to_string(),
        VK_END => "End".to_string(),
        VK_UP => "ArrowUp".to_string(),
        VK_DOWN => "ArrowDown".to_string(),
        VK_LEFT => "ArrowLeft".to_string(),
        VK_RIGHT => "ArrowRight".to_string(),
        VK_ADD => "NumpadAdd".to_string(),
        VK_SUBTRACT => "NumpadSubtract".to_string(),
        VK_MULTIPLY => "NumpadMultiply".to_string(),
        VK_DIVIDE => "NumpadDivide".to_string(),
        VK_OEM_4 => "BracketLeft".to_string(),
        VK_OEM_6 => "BracketRight".to_string(),
        VK_OEM_1 => "Semicolon".to_string(),
        VK_OEM_7 => "Quote".to_string(),
        VK_OEM_3 => "Backquote".to_string(),
        VK_OEM_5 => "Backslash".to_string(),
        VK_OEM_COMMA => "Comma".to_string(),
        VK_OEM_PERIOD => "Period".to_string(),
        VK_OEM_2 => "Slash".to_string(),
        _ => format!("Unknown_0x{:X}", vk),
    }
}

/// Builds a canonical shortcut string (`"super+ctrl+alt+shift+<key>"`).
pub fn build_canonical_string(modifiers: u32, key_token: &str) -> String {
    let mut parts = Vec::new();
    if (modifiers & MOD_WIN) != 0 {
        parts.push("super");
    }
    if (modifiers & MOD_CONTROL) != 0 {
        parts.push("ctrl");
    }
    if (modifiers & MOD_ALT) != 0 {
        parts.push("alt");
    }
    if (modifiers & MOD_SHIFT) != 0 {
        parts.push("shift");
    }
    parts.push(key_token);
    parts.join("+")
}

/// Parses a shortcut string into its Win32 modifiers and VK code with validation.
pub fn parse_shortcut(shortcut_str: &str) -> Result<ParsedShortcut, ShortcutError> {
    let trimmed = shortcut_str.trim();
    if trimmed.is_empty() {
        return Err(ShortcutError::Empty);
    }

    let parts: Vec<&str> = trimmed.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err(ShortcutError::Empty);
    }

    let mut modifiers = 0u32;
    let key_token = parts[parts.len() - 1];

    for part in &parts[..parts.len() - 1] {
        match part.to_ascii_lowercase().as_str() {
            "super" | "win" | "meta" => modifiers |= MOD_WIN,
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            _ => {}
        }
    }

    if !is_safe_modifier(modifiers) {
        return Err(ShortcutError::UnsafeModifier);
    }

    let vk = key_token_to_vk(key_token)
        .ok_or_else(|| ShortcutError::UnknownKey(key_token.to_string()))?;

    let canonical = build_canonical_string(modifiers, key_token);

    Ok(ParsedShortcut {
        modifiers,
        vk,
        key_token: key_token.to_string(),
        canonical_string: canonical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_shortcuts() {
        let pin = parse_shortcut("super+ctrl+KeyT").unwrap();
        assert_eq!(pin.modifiers, MOD_WIN | MOD_CONTROL);
        assert_eq!(pin.vk, b'T' as u32);
        assert_eq!(pin.canonical_string, "super+ctrl+KeyT");

        let op_up = parse_shortcut("super+ctrl+Equal").unwrap();
        assert_eq!(op_up.modifiers, MOD_WIN | MOD_CONTROL);
        assert_eq!(op_up.vk, VK_OEM_PLUS);

        let op_down = parse_shortcut("super+ctrl+Minus").unwrap();
        assert_eq!(op_down.modifiers, MOD_WIN | MOD_CONTROL);
        assert_eq!(op_down.vk, VK_OEM_MINUS);

        let win = parse_shortcut("super+ctrl+KeyP").unwrap();
        assert_eq!(win.modifiers, MOD_WIN | MOD_CONTROL);
        assert_eq!(win.vk, b'P' as u32);
    }

    #[test]
    fn test_canonical_reordering() {
        // "ctrl+super+KeyT" should reorder to canonical "super+ctrl+KeyT"
        let parsed = parse_shortcut("ctrl+super+KeyT").unwrap();
        assert_eq!(parsed.canonical_string, "super+ctrl+KeyT");

        // "shift+alt+ctrl+super+KeyA" -> "super+ctrl+alt+shift+KeyA"
        let parsed2 = parse_shortcut("shift+alt+ctrl+super+KeyA").unwrap();
        assert_eq!(parsed2.canonical_string, "super+ctrl+alt+shift+KeyA");
    }

    #[test]
    fn test_safe_modifier_rule() {
        // Safe modifiers
        assert!(parse_shortcut("super+KeyA").is_ok());
        assert!(parse_shortcut("ctrl+KeyA").is_ok());
        assert!(parse_shortcut("alt+KeyA").is_ok());
        assert!(parse_shortcut("ctrl+shift+KeyA").is_ok());

        // Unsafe: Shift alone or bare key must fail
        assert_eq!(
            parse_shortcut("shift+KeyA"),
            Err(ShortcutError::UnsafeModifier)
        );
        assert_eq!(
            parse_shortcut("KeyA"),
            Err(ShortcutError::UnsafeModifier)
        );
        assert_eq!(
            parse_shortcut(""),
            Err(ShortcutError::Empty)
        );
    }

    #[test]
    fn test_vk_to_token_roundtrip() {
        assert_eq!(vk_to_key_token(b'T' as u32), "KeyT");
        assert_eq!(vk_to_key_token(b'A' as u32), "KeyA");
        assert_eq!(vk_to_key_token(VK_OEM_PLUS), "Equal");
        assert_eq!(vk_to_key_token(VK_OEM_MINUS), "Minus");
        assert_eq!(vk_to_key_token(VK_F9), "F9");
        assert_eq!(vk_to_key_token(VK_SPACE), "Space");
    }
}
