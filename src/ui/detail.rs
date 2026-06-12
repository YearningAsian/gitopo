use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::git::format_relative_time_with_now;

use super::theme::{COLOR_BORDER_UNFOCUSED, COLOR_DIM, COLOR_LOCAL, COLOR_TAG, COLOR_TITLE};

pub fn render_detail_pane(frame: &mut Frame, app: &App, area: Rect, now: i64) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_UNFOCUSED))
        .title(Span::styled(
            " Commit Details ",
            Style::default().fg(COLOR_TITLE),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(oid) = app.selected_commit_oid() else {
        return;
    };
    let Some(commit) = app.repo_data.commits.get(&oid) else {
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("commit  ", Style::default().fg(COLOR_DIM)),
        Span::styled(
            format!("{}", oid),
            Style::default().fg(Color::Rgb(255, 200, 100)),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("author  ", Style::default().fg(COLOR_DIM)),
        Span::styled(commit.author.clone(), Style::default().fg(COLOR_LOCAL)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("date    ", Style::default().fg(COLOR_DIM)),
        Span::styled(
            format_relative_time_with_now(commit.time, now),
            Style::default().fg(COLOR_DIM),
        ),
    ]));

    if !commit.parent_oids.is_empty() {
        let parents: Vec<String> = commit
            .parent_oids
            .iter()
            .map(|p| format!("{:.7}", p))
            .collect();
        lines.push(Line::from(vec![
            Span::styled("parents ", Style::default().fg(COLOR_DIM)),
            Span::styled(parents.join("  "), Style::default().fg(COLOR_DIM)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        commit.message.clone(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(labels) = app.repo_data.oid_to_branches.get(&oid) {
        lines.push(Line::from(""));
        let label_spans: Vec<Span> = labels
            .iter()
            .flat_map(|l| {
                vec![
                    Span::styled(format!("[{}]", l), Style::default().fg(COLOR_TAG)),
                    Span::raw(" "),
                ]
            })
            .collect();
        lines.push(Line::from(label_spans));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}
