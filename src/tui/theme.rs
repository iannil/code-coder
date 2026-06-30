/// ─── Theme (ADR 0003) ──────────────────────────────────────────────────────
///
/// Central color definition for the TUI. Every render function reads from
/// `app.theme.<role>`; no `Color::X` literals are allowed in render code.
/// Adding a new visual element = adding a new role field here, not picking
/// a color inline.
///
/// Roles are named by **semantic purpose** (primary_text, warning), not by
/// literal color (White, Yellow). The Theme decides what color each role
/// maps to; render code is unaware.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // ── Text roles ───────────────────────────────────────────────────────
    /// Main content text (user messages, dialog tool name, help keys).
    pub primary_text: Color,
    /// Dimmed text (system msgs marked [end]/[error], reasoning CoT,
    /// tool I/O, separators, status bar secondary).
    pub secondary_text: Color,
    /// Accent / emphasis (model name in status bar, "> " prompt, message
    /// prefixes ▶ ▷, Cyan section headers).
    pub accent_text: Color,
    /// Warning (destructive-op marks, yellow dialog borders, [!] headers).
    pub warning_text: Color,
    /// Success / "current" indicator (✓ next to active model in picker).
    pub success_text: Color,

    // ── Selection (popups, model picker, slash completion) ───────────────
    /// Foreground color of the highlighted/selected row.
    pub selected_fg: Color,
    /// Background color of the highlighted/selected row.
    pub selected_bg: Color,

    // ── Markdown rendering (passed to markdown::set_theme_colors) ────────
    pub markdown_emphasis: Color,
    pub markdown_link: Color,
    pub markdown_code: Color,
    pub markdown_quote: Color,
}

impl Theme {
    /// Default dark theme — the original pre-ADR-0003 look.
    pub fn dark() -> Self {
        Self {
            primary_text: Color::White,
            secondary_text: Color::DarkGray,
            accent_text: Color::Cyan,
            warning_text: Color::Yellow,
            success_text: Color::Green,
            selected_fg: Color::Black,
            selected_bg: Color::White,
            markdown_emphasis: Color::Yellow,
            markdown_link: Color::Cyan,
            markdown_code: Color::Green,
            markdown_quote: Color::DarkGray,
        }
    }

    /// Light theme — high-contrast inversion for bright environments.
    pub fn light() -> Self {
        Self {
            primary_text: Color::Black,
            secondary_text: Color::Gray,
            accent_text: Color::Blue,
            warning_text: Color::Red,
            success_text: Color::Green,
            selected_fg: Color::White,
            selected_bg: Color::Black,
            markdown_emphasis: Color::Red,
            markdown_link: Color::Blue,
            markdown_code: Color::Green,
            markdown_quote: Color::Gray,
        }
    }

    /// Convenience: true when this theme is the dark variant. Used by the
    /// scrollbar thumb and any code that needs a binary "is dark" check
    /// without comparing individual fields.
    pub fn is_dark(&self) -> bool {
        self.primary_text == Color::White
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_returns_dark_variant() {
        let t = Theme::dark();
        assert!(t.is_dark());
        assert_eq!(t.primary_text, Color::White);
    }

    #[test]
    fn light_theme_returns_light_variant() {
        let t = Theme::light();
        assert!(!t.is_dark());
        assert_eq!(t.primary_text, Color::Black);
    }

    #[test]
    fn dark_and_light_differ_on_primary_text() {
        // The two themes must produce visibly different output.
        assert_ne!(Theme::dark().primary_text, Theme::light().primary_text);
    }

    #[test]
    fn dark_and_light_differ_on_selected_colors() {
        // Selection inverts in light mode.
        let d = Theme::dark();
        let l = Theme::light();
        assert_ne!(d.selected_fg, l.selected_fg);
        assert_ne!(d.selected_bg, l.selected_bg);
    }

    #[test]
    fn default_is_dark() {
        // App starts in dark mode (matches prior dark_mode = true default).
        assert_eq!(Theme::default().primary_text, Theme::dark().primary_text);
    }
}
