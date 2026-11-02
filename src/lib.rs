//! Cube Escape: rotate a cube and slide its arrows away, one clear path at a time.

pub mod game;
pub mod icon;
pub mod puzzle;
pub mod scene;

/// The window title and display name. Change it here if the game is renamed.
pub const GAME_TITLE: &str = "Cube Escape";

/// The program's identity to the desktop: the X11 window class (and Wayland
/// app id), and the name of its `.desktop` file and icon. Panels and menus
/// use it to match a running window to its icon. Keep it in step with
/// `packaging/cube-escape.desktop.in`; a test checks that.
pub const APP_ID: &str = "cube-escape";

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP_FILE: &str = include_str!("../packaging/cube-escape.desktop.in");
    const INSTALL_SCRIPT: &str = include_str!("../packaging/install-local.sh");

    fn field<'a>(key: &str) -> Option<&'a str> {
        DESKTOP_FILE
            .lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
    }

    #[test]
    fn the_app_id_is_a_safe_file_and_class_name() {
        assert!(!APP_ID.is_empty());
        assert!(APP_ID
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        // The default would be shared with every other miniquad program.
        assert_ne!(APP_ID, "miniquad-application");
    }

    #[test]
    fn the_desktop_file_matches_the_window_class_and_icon_name() {
        assert_eq!(field("StartupWMClass"), Some(APP_ID));
        assert_eq!(field("Icon"), Some(APP_ID));
        assert_eq!(field("Name"), Some(GAME_TITLE));
        assert_eq!(field("Type"), Some("Application"));
        assert!(DESKTOP_FILE.starts_with("[Desktop Entry]"));
        assert!(field("Categories").is_some_and(|c| c.contains("Game") && c.ends_with(';')));
        assert!(field("Exec").is_some_and(|e| e.contains("@EXEC@")));
    }

    #[test]
    fn the_install_script_uses_the_same_app_id() {
        assert!(INSTALL_SCRIPT.contains(&format!("APP_ID={APP_ID}\n")));
    }
}
