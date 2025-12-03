use once_cell::sync::Lazy;
use ratatui::style::Color;

/// Detected or configured theme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    /// Parse theme from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

/// Lazily detected theme
static DETECTED_THEME: Lazy<Theme> = Lazy::new(detect_terminal_theme);

/// Detect the terminal's color scheme (light or dark)
fn detect_terminal_theme() -> Theme {
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
pub fn get_theme(theme_arg: Option<&str>) -> Theme {
    match theme_arg {
        Some(s) => Theme::from_str(s).unwrap_or_else(detected_theme),
        None => detected_theme(),
    }
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
    /// Search highlight background
    pub search_bg: Color,
    /// Search highlight foreground
    pub search_fg: Color,
    /// Match line highlight background
    pub match_line_bg: Color,
    /// Context line color
    pub context_fg: Color,
    /// Separator color
    pub separator: Color,
    /// Error message color
    pub error: Color,
}

impl ThemeColors {
    /// Get colors for the given theme
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::light(),
            Theme::Dark => Self::dark(),
        }
    }

    /// Light theme colors
    fn light() -> Self {
        Self {
            line_number: Color::DarkGray,
            status_bg: Color::Rgb(200, 200, 200),
            status_fg: Color::Black,
            search_bg: Color::Yellow,
            search_fg: Color::Black,
            match_line_bg: Color::Rgb(255, 255, 200),
            context_fg: Color::DarkGray,
            separator: Color::DarkGray,
            error: Color::Red,
        }
    }

    /// Dark theme colors
    fn dark() -> Self {
        Self {
            line_number: Color::DarkGray,
            status_bg: Color::DarkGray,
            status_fg: Color::White,
            search_bg: Color::Yellow,
            search_fg: Color::Black,
            match_line_bg: Color::Rgb(50, 50, 30),
            context_fg: Color::DarkGray,
            separator: Color::DarkGray,
            error: Color::Red,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_from_str() {
        assert_eq!(Theme::from_str("light"), Some(Theme::Light));
        assert_eq!(Theme::from_str("LIGHT"), Some(Theme::Light));
        assert_eq!(Theme::from_str("dark"), Some(Theme::Dark));
        assert_eq!(Theme::from_str("DARK"), Some(Theme::Dark));
        assert_eq!(Theme::from_str("invalid"), None);
    }

    #[test]
    fn test_theme_colors() {
        let light = ThemeColors::for_theme(Theme::Light);
        let dark = ThemeColors::for_theme(Theme::Dark);

        // Both should have yellow search background
        assert_eq!(light.search_bg, Color::Yellow);
        assert_eq!(dark.search_bg, Color::Yellow);

        // Status bar should be different
        assert_ne!(light.status_bg, dark.status_bg);
    }
}
