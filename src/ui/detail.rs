use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::git::format_relative_time_with_now;

pub fn render_detail_pane(frame: &mut Frame, app: &App, area: Rect, now: i64) {
    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_unfocused))
        .title(Span::styled(
            " Commit Details ",
            Style::default().fg(theme.title),
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
        Span::styled("commit  ", Style::default().fg(theme.dim)),
        Span::styled(format!("{}", oid), Style::default().fg(theme.hash)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("author  ", Style::default().fg(theme.dim)),
        Span::styled(commit.author.clone(), Style::default().fg(theme.local)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("date    ", Style::default().fg(theme.dim)),
        Span::styled(
            format_relative_time_with_now(commit.time, now),
            Style::default().fg(theme.dim),
        ),
    ]));

    if !commit.parent_oids.is_empty() {
        let parents: Vec<String> = commit
            .parent_oids
            .iter()
            .map(|p| format!("{:.7}", p))
            .collect();
        lines.push(Line::from(vec![
            Span::styled("parents ", Style::default().fg(theme.dim)),
            Span::styled(parents.join("  "), Style::default().fg(theme.dim)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        commit.message.clone(),
        Style::default()
            .fg(theme.text_strong)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(labels) = app.repo_data.oid_to_branches.get(&oid) {
        lines.push(Line::from(""));
        let label_spans: Vec<Span> = labels
            .iter()
            .flat_map(|l| {
                vec![
                    Span::styled(format!("[{}]", l), Style::default().fg(theme.tag)),
                    Span::raw(" "),
                ]
            })
            .collect();
        lines.push(Line::from(label_spans));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}
