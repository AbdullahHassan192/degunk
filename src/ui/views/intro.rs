use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

pub struct IntroState {
    pub start_time: Instant,
    pub duration: Duration,
}

impl IntroState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(1000),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            1.0
        } else {
            (elapsed / total).clamp(0.0, 1.0)
        }
    }
}

impl Default for IntroState {
    fn default() -> Self {
        Self::new()
    }
}

const LOGO_LINES: &[&str] = &[
    r"██████╗  ███████╗  ██████╗  ██╗   ██╗ ███╗   ██╗ ██╗  ██╗",
    r"██╔══██╗ ██╔════╝ ██╔════╝  ██║   ██║ ████╗  ██║ ██║ ██╔╝",
    r"██║  ██║ █████╗   ██║  ███╗ ██║   ██║ ██╔██╗ ██║ █████═╝ ",
    r"██║  ██║ ██╔══╝   ██║   ██║ ██║   ██║ ██║╚██╗██║ ██╔═██╗ ",
    r"██████╔╝ ███████╗ ╚██████╔╝ ╚██████╔╝ ██║ ╚████║ ██║  ██╗",
    r"╚═════╝  ╚══════╝  ╚═════╝   ╚═════╝  ╚═╝  ╚═══╝ ╚═╝  ╚═╝",
];

pub fn render_intro(f: &mut Frame, intro: &IntroState) {
    let area = f.area();
    f.render_widget(Clear, area);

    let progress = intro.progress();
    let mut lines = Vec::new();

    let logo_width = 58;
    let logo_height = LOGO_LINES.len();

    // Check if terminal has enough room for full ASCII banner
    if area.width >= 62 && area.height >= 12 {
        let total_pad = (area.height as usize).saturating_sub(logo_height + 5) / 2;
        for _ in 0..total_pad {
            lines.push(Line::from(""));
        }

        // The sweep beam completes across the banner by ~58% of the 1s duration
        let sweep_progress = (progress / 0.58).min(1.0);
        let sweep_x = sweep_progress * (logo_width as f32 + 20.0) - 10.0;

        for line_str in LOGO_LINES {
            let mut spans = Vec::new();
            let chars: Vec<char> = line_str.chars().collect();

            // Calculate centering indent
            let indent_spaces = (area.width as usize).saturating_sub(logo_width) / 2;
            if indent_spaces > 0 {
                spans.push(Span::raw(" ".repeat(indent_spaces)));
            }

            for (col, &ch) in chars.iter().enumerate() {
                let dist = (col as f32) - sweep_x;

                let style = if dist > 2.5 {
                    // Ahead of beam: dim slate
                    Style::default().fg(Color::Rgb(50, 60, 75))
                } else if dist.abs() <= 1.5 {
                    // Peak center of beam: brilliant white
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if dist < -1.5 && dist >= -4.5 {
                    // Trailing soft glow: bright electric cyan
                    Style::default()
                        .fg(Color::Rgb(120, 225, 255))
                        .add_modifier(Modifier::BOLD)
                } else if dist < -4.5 && dist >= -8.0 {
                    // Settling into brand cyan
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    // Settled brand cyan
                    Style::default().fg(Color::Cyan)
                };

                spans.push(Span::styled(ch.to_string(), style));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        // Subtitle line with sparkle
        if progress >= 0.35 {
            let subtitle_alpha = ((progress - 0.35) / 0.23).min(1.0);
            let sparkle_color = if subtitle_alpha > 0.8 {
                Color::Rgb(255, 220, 100) // Light golden sparkle
            } else {
                Color::Cyan
            };

            let text_color = if subtitle_alpha > 0.6 {
                Color::White
            } else {
                Color::DarkGray
            };

            lines.push(
                Line::from(vec![
                    Span::styled("✦  ", Style::default().fg(sparkle_color).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "reclaim your developer disk space",
                        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  •  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
                ])
                .alignment(Alignment::Center),
            );
        } else {
            lines.push(Line::from(""));
        }
    } else {
        // Compact fallback for narrow terminals
        let total_pad = (area.height as usize).saturating_sub(4) / 2;
        for _ in 0..total_pad {
            lines.push(Line::from(""));
        }

        let title = "✦  D E G U N K  v0.1.0  ✦";
        let sweep_x = (progress / 0.58).min(1.0) * (title.len() as f32 + 10.0) - 5.0;

        let mut spans = Vec::new();
        for (col, ch) in title.chars().enumerate() {
            let dist = (col as f32) - sweep_x;
            let color = if dist > 2.0 {
                Color::DarkGray
            } else if dist.abs() <= 2.0 {
                Color::White
            } else {
                Color::Cyan
            };
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(spans).alignment(Alignment::Center));
        lines.push(Line::from(""));
        lines.push(
            Line::from(Span::styled(
                "reclaim your disk",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Center),
        );
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);

    // Subtle skip hint at bottom
    if area.height >= 4 {
        let footer_rect = Rect::new(area.x, area.y + area.height - 2, area.width, 1);
        let footer_p = Paragraph::new(Line::from(Span::styled(
            "press any key to skip",
            Style::default().fg(Color::Rgb(75, 85, 105)),
        )))
        .alignment(Alignment::Center);
        f.render_widget(footer_p, footer_rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_intro_state_progress() {
        let state = IntroState::new();
        assert!(!state.is_finished());
        assert!(state.progress() >= 0.0 && state.progress() <= 1.0);
    }

    #[test]
    fn test_render_shimmer_no_panic() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = IntroState::new();
        terminal
            .draw(|f| {
                render_intro(f, &state);
            })
            .unwrap();

        // Narrow screen fallback
        let small_backend = TestBackend::new(40, 10);
        let mut small_terminal = Terminal::new(small_backend).unwrap();
        small_terminal
            .draw(|f| {
                render_intro(f, &state);
            })
            .unwrap();
    }
}
