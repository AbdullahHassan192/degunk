use ratatui::style::{Color, Modifier, Style};
use crate::core::ecosystem::Ecosystem;

pub struct Theme;

impl Theme {
    // Primary UI colors
    pub const ACCENT: Color = Color::Cyan;
    pub const ACCENT_ALT: Color = Color::Magenta;
    pub const BG_HEADER: Color = Color::Rgb(20, 24, 34);
    pub const TEXT_PRIMARY: Color = Color::White;
    pub const TEXT_MUTED: Color = Color::DarkGray;
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const DANGER: Color = Color::Red;
    pub const INFO: Color = Color::LightBlue;

    pub fn title_style() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row_style() -> Style {
        Style::default()
            .bg(Color::Rgb(35, 45, 65))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_row_style() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn ecosystem_color(eco: Ecosystem) -> Color {
        match eco {
            Ecosystem::Node => Color::Rgb(104, 160, 99),     // Node green
            Ecosystem::Python => Color::Rgb(53, 114, 165),   // Python blue
            Ecosystem::Rust => Color::Rgb(222, 90, 53),      // Rust orange
            Ecosystem::Java => Color::Rgb(176, 114, 25),     // Java golden
            Ecosystem::Go => Color::Rgb(0, 173, 216),        // Go cyan
            Ecosystem::DotNet => Color::Rgb(81, 43, 212),    // .NET purple
            Ecosystem::Cpp => Color::Rgb(243, 75, 125),      // C++ pink
            Ecosystem::Swift => Color::Rgb(250, 115, 51),    // Swift orange
            Ecosystem::Flutter => Color::Rgb(2, 86, 155),    // Flutter light blue
            Ecosystem::Php => Color::Rgb(119, 123, 179),     // PHP indigo
            Ecosystem::Elixir => Color::Rgb(110, 74, 126),   // Elixir violet
            Ecosystem::Zig => Color::Rgb(247, 164, 29),      // Zig amber
            Ecosystem::Godot => Color::Rgb(71, 140, 194),    // Godot blue
            Ecosystem::Unity => Color::Rgb(34, 44, 55),      // Unity dark slate
            Ecosystem::Coverage => Color::Rgb(38, 166, 154), // Coverage teal
        }
    }

    pub fn size_color(bytes: u64) -> Color {
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * MB;

        if bytes >= 2 * GB {
            Color::Red
        } else if bytes >= 500 * MB {
            Color::LightRed
        } else if bytes >= 100 * MB {
            Color::Yellow
        } else {
            Color::Green
        }
    }
}
