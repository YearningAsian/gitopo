mod branch;
mod commit;
mod detail;
mod overlay;
pub mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Focus};

use self::theme::{truncate_path, COLOR_DIM, COLOR_TITLE};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let now = chrono::Utc::now().timestamp();

    if app.focus == Focus::Help {
        overlay::render_help(frame, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_title_bar(frame, app, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[1]);

    branch::render_branch_list(frame, app, main_chunks[0], now);
    commit::render_commit_graph(frame, app, right_chunks[0], now);
    detail::render_detail_pane(frame, app, right_chunks[1], now);
    render_status_bar(frame, app, chunks[2]);

    if app.search_mode {
        overlay::render_search_overlay(frame, app, area);
    }
}

fn render_title_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let repo_display = truncate_path(
        &app.repo_data.repo_path,
        (area.width as usize).saturating_sub(20),
    );
    let mode = if app.show_all { " [all]" } else { " [local]" };
    let title = format!(" gitopo  {}{}  ", repo_display, mode);
    let paragraph = Paragraph::new(title).style(
        Style::default()
            .fg(COLOR_TITLE)
            .bg(Color::Rgb(25, 28, 38))
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let msg = if let Some(ref s) = app.status_msg {
        s.clone()
    } else {
        let search_hint = if !app.search_query.is_empty() {
            format!("  search: \"{}\"", app.search_query)
        } else {
            String::new()
        };
        format!(
            " [?] help  [/] search  [a] toggle all  [r] refresh  [q] quit{}",
            search_hint
        )
    };

    frame.render_widget(
        Paragraph::new(msg).style(Style::default().fg(COLOR_DIM).bg(Color::Rgb(20, 22, 30))),
        area,
    );
}
