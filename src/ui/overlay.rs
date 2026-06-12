use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

use super::theme::{COLOR_BORDER_FOCUSED, COLOR_LOCAL, COLOR_TITLE};

pub fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    const POPUP_HEIGHT: u16 = 3;
    if area.height < POPUP_HEIGHT + 1 || area.width < 8 {
        return;
    }
    let popup_width = area.width.min(60);
    let popup_x = area.x + (area.width - popup_width) / 2;
    let popup_y = area.y + area.height - POPUP_HEIGHT - 1;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: POPUP_HEIGHT,
    };

    frame.render_widget(Clear, popup_area);

    let match_info = if app.search_matches.is_empty() {
        " (no matches)".to_owned()
    } else {
        format!(
            " ({}/{})",
            app.search_match_idx + 1,
            app.search_matches.len()
        )
    };

    let text = format!("/{}{}", app.search_query, match_info);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_FOCUSED))
        .title(Span::styled(" Search ", Style::default().fg(COLOR_TITLE)));

    frame.render_widget(Paragraph::new(text).block(block), popup_area);
}

pub fn render_help(frame: &mut Frame, area: Rect) {
    let popup_width = area.width.min(60);
    let popup_height = area.height.min(22);
    let popup_x = area.x + (area.width - popup_width) / 2;
    let popup_y = area.y + (area.height - popup_height) / 2;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let lines: Vec<Line> = vec![
        Line::from(""),
        keybind_line("j / ↓", "Move down"),
        keybind_line("k / ↑", "Move up"),
        keybind_line("Ctrl+f / PgDn", "Page down"),
        keybind_line("Ctrl+b / PgUp", "Page up"),
        keybind_line("g / Home", "Jump to top"),
        keybind_line("G / End", "Jump to bottom"),
        keybind_line("Enter / l / →", "Focus commit graph"),
        keybind_line("h / Esc / ←", "Back to branch list"),
        Line::from(""),
        keybind_line("/", "Search branches"),
        keybind_line("n / N", "Next / prev search match"),
        keybind_line("a", "Toggle local / all branches"),
        keybind_line("r", "Refresh repository data"),
        Line::from(""),
        keybind_line("?", "Toggle this help"),
        keybind_line("q / Ctrl+C", "Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  gitopo — read-only branch topology explorer",
            Style::default().fg(Color::Rgb(100, 100, 120)),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_FOCUSED))
        .title(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(COLOR_TITLE)
                .add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn keybind_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:18}", key),
            Style::default()
                .fg(COLOR_LOCAL)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}
