pub mod chat_screen;
pub mod settings_screen;

/// Color theme constants - Catppuccin-inspired
pub mod colors {
    use ratatui::style::Color;

    // Catppuccin Mocha palette - Unused colors are prefixed with underscore
    pub const MAUVE: Color = Color::Rgb(197, 148, 255);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    pub const SKY: Color = Color::Rgb(137, 220, 235);
    pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT_1: Color = Color::Rgb(186, 194, 222);
    pub const SUBTEXT_0: Color = Color::Rgb(166, 173, 200);
    pub const OVERLAY_1: Color = Color::Rgb(127, 132, 156);
    pub const OVERLAY_0: Color = Color::Rgb(108, 112, 134);
    pub const SURFACE_2: Color = Color::Rgb(88, 91, 112);
    pub const SURFACE_0: Color = Color::Rgb(49, 50, 68);
    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const MANTLE: Color = Color::Rgb(24, 24, 37);
    pub const CRUST: Color = Color::Rgb(17, 17, 27);
}

/// Screen variants for the application
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Chat,
    Settings,
    Quit,
}

/// Format a timestamp for display
pub fn format_timestamp(time: &str) -> String {
    format!("🕐 {}", time)
}

/// Sanitize AI response text by removing control characters that display as garbled symbols
pub fn sanitize_text(text: &str) -> String {
    text.chars()
        .filter(|&c| {
            // Keep printable chars, newlines, carriage returns, tabs
            if c == '\n' || c == '\r' || c == '\t' {
                return true;
            }
            // Keep normal printable ASCII and Unicode
            if c.is_ascii_graphic() || c.is_ascii_whitespace() {
                return true;
            }
            // Keep Unicode letters, marks, numbers, punctuation, symbols
            if c.is_alphanumeric() || c.is_ascii_punctuation() {
                return true;
            }
            // Allow common Unicode categories
            let cat = std::char::UNICODE_VERSION;
            c > '\u{00A0}' && !c.is_control()
        })
        .collect::<String>()
        .replace('\t', "    ")
}

