use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Mode};

/// Handle a key event, returning true if the app should quit
pub fn handle_key(key: KeyEvent, app: &mut App) -> bool {
    // Check for Ctrl+C first - always quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return true;
    }

    // Handle based on current mode
    match &app.mode {
        Mode::Normal => handle_normal_mode(key, app),
        Mode::Search { .. } => handle_search_mode(key, app),
    }
}

/// Handle key events in normal mode
fn handle_normal_mode(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
            true
        }

        // Enter search mode (case-insensitive)
        KeyCode::Char('/') => {
            app.enter_search_mode(true);
            false
        }

        // Enter search mode (case-sensitive)
        KeyCode::Char('?') => {
            app.enter_search_mode(false);
            false
        }

        // Scroll down
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_down(1);
            false
        }

        // Scroll up
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_up(1);
            false
        }

        // Scroll left
        KeyCode::Char('h') | KeyCode::Left => {
            app.scroll_left(4);
            false
        }

        // Scroll right
        KeyCode::Char('l') | KeyCode::Right => {
            app.scroll_right(4);
            false
        }

        // Half page down
        KeyCode::Char('d') | KeyCode::PageDown => {
            app.scroll_half_page_down();
            false
        }

        // Half page up
        KeyCode::Char('u') | KeyCode::PageUp => {
            app.scroll_half_page_up();
            false
        }

        // Go to line start
        KeyCode::Char('0') => {
            app.scroll_to_line_start();
            false
        }

        // Go to line end
        KeyCode::Char('$') => {
            app.scroll_to_line_end();
            false
        }

        // Go to top
        KeyCode::Char('g') | KeyCode::Home => {
            app.go_to_top();
            false
        }

        // Go to bottom
        KeyCode::Char('G') | KeyCode::End => {
            app.go_to_bottom();
            false
        }

        // Next search match
        KeyCode::Char('n') => {
            app.next_match();
            false
        }

        // Previous search match
        KeyCode::Char('N') => {
            app.prev_match();
            false
        }

        // Toggle follow mode
        KeyCode::Char('f') => {
            app.toggle_follow();
            false
        }

        // Toggle line numbers
        KeyCode::Char('#') => {
            app.show_line_numbers = !app.show_line_numbers;
            false
        }

        _ => false,
    }
}

/// Handle key events in search mode
fn handle_search_mode(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        // Cancel search
        KeyCode::Esc => {
            app.cancel_search();
            false
        }

        // Confirm search
        KeyCode::Enter => {
            app.confirm_search();
            false
        }

        // Delete last character
        KeyCode::Backspace => {
            app.search_backspace();
            false
        }

        // Add character to search query
        KeyCode::Char(c) => {
            app.search_add_char(c);
            false
        }

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::WrapMode;
    use crate::display::Document;
    use crate::theme::{Theme, ThemeColors};

    fn create_test_app() -> App {
        let doc = Document::from_text(
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5",
            "test.txt".to_string(),
            "UTF-8".to_string(),
        );
        let theme_colors = ThemeColors::for_theme(Theme::Dark);
        let mut app = App::new(doc, false, None, theme_colors, false, None, WrapMode::None, 200);
        app.set_terminal_size(80, 3); // 2 content lines visible
        app
    }

    #[test]
    fn test_quit_keys() {
        let mut app = create_test_app();

        // 'q' should quit
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(handle_key(key, &mut app));
        assert!(app.should_quit);
    }

    #[test]
    fn test_scroll_down() {
        let mut app = create_test_app();

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        handle_key(key, &mut app);
        assert_eq!(app.scroll_line, 1);
    }

    #[test]
    fn test_scroll_up() {
        let mut app = create_test_app();
        app.scroll_line = 2;

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        handle_key(key, &mut app);
        assert_eq!(app.scroll_line, 1);
    }

    #[test]
    fn test_go_to_top_bottom() {
        let mut app = create_test_app();
        app.scroll_line = 2;

        // Go to top
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        handle_key(key, &mut app);
        assert_eq!(app.scroll_line, 0);

        // Go to bottom
        let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE);
        handle_key(key, &mut app);
        assert_eq!(app.scroll_line, 3); // 5 lines - 2 visible = 3
    }
}
