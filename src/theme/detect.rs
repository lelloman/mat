use std::sync::LazyLock;

use clap::ValueEnum;
use ratatui::style::Color;

/// Detected or configured theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

/// Lazily detected theme
static DETECTED_THEME: LazyLock<Theme> = LazyLock::new(detect_terminal_theme);

/// Detect the terminal's color scheme (light or dark)
fn detect_terminal_theme() -> Theme {
    use std::io::IsTerminal;

    // Skip terminal detection if stdout is not a TTY (e.g., in tests or pipes)
    // This prevents terminal escape sequences from corrupting non-TTY streams
    if !std::io::stdout().is_terminal() {
        return Theme::Dark;
    }

    // Also skip if TERM is "dumb" (common in test environments)
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return Theme::Dark;
    }

    // Try using terminal-light to detect the background
    match terminal_light::background_color() {
        Ok(color) => {
            // Convert to RGB to get the components
            let rgb = color.rgb();

            // Calculate luminance
            let r = rgb.r as f64 / 255.0;
            let g = rgb.g as f64 / 255.0;
            let b = rgb.b as f64 / 255.0;

            // Use relative luminance formula
            let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;

            if luminance > 0.5 {
                Theme::Light
            } else {
                Theme::Dark
            }
        }
        Err(_) => {
            // Fallback to dark theme if detection fails
            Theme::Dark
        }
    }
}

/// Get the detected theme
pub fn detected_theme() -> Theme {
    *DETECTED_THEME
}

/// Get theme from CLI arg or auto-detect
pub fn get_theme(theme_arg: Option<Theme>) -> Theme {
    theme_arg.unwrap_or_else(detected_theme)
}

/// Color scheme for the UI
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Line number color
    pub line_number: Color,
    /// Status bar background
    pub status_bg: Color,
    /// Status bar foreground
    pub status_fg: Color,
}

impl ThemeColors {
    /// Get colors for the given theme
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::light(),
            Theme::Dark => Self::dark(),
        }
    }

    /// A style-free palette for `--color never` and `NO_COLOR`.
    pub fn plain() -> Self {
        Self {
            line_number: Color::Reset,
            status_bg: Color::Reset,
            status_fg: Color::Reset,
        }
    }

    /// Light theme colors
    fn light() -> Self {
        Self {
            line_number: Color::DarkGray,
            status_bg: Color::Rgb(200, 200, 200),
            status_fg: Color::Black,
        }
    }

    /// Dark theme colors
    fn dark() -> Self {
        Self {
            line_number: Color::DarkGray,
            status_bg: Color::DarkGray,
            status_fg: Color::White,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_from_str() {
        use clap::ValueEnum;
        assert_eq!(Theme::from_str("light", true), Ok(Theme::Light));
        assert_eq!(Theme::from_str("LIGHT", true), Ok(Theme::Light));
        assert_eq!(Theme::from_str("dark", true), Ok(Theme::Dark));
        assert!(Theme::from_str("invalid", true).is_err());
    }

    #[test]
    fn test_theme_colors() {
        let light = ThemeColors::for_theme(Theme::Light);
        let dark = ThemeColors::for_theme(Theme::Dark);

        // Status bar should be different
        assert_ne!(light.status_bg, dark.status_bg);
    }
}
