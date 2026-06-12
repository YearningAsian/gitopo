use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

use crate::app::{App, Focus};
use crate::git::{format_relative_time_with_now, CommitInfo};
use crate::graph::render_graph_prefix;

use super::theme::{
    truncate_str, COLOR_BORDER_FOCUSED, COLOR_BORDER_UNFOCUSED, COLOR_DIM, COLOR_GRAPH_LINE,
    COLOR_GRAPH_NODE, COLOR_SELECTED_BG, COLOR_TAG, COLOR_TITLE,
};

pub fn render_commit_graph(frame: &mut Frame, app: &mut App, area: Rect, now: i64) {
    let inner_height = area.height.saturating_sub(2) as usize;
    app.scroll_graph(inner_height);

    let focused = app.focus == Focus::CommitGraph;
    let border_color = if focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_UNFOCUSED
    };

    let branch_name = app
        .selected_branch()
        .map(|b| b.name.clone())
        .unwrap_or_default();
    let commit_count = app.active_branch_oids.len();
    let title = format!(" {} ({} commits) ", branch_name, commit_count);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(COLOR_TITLE)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_start = app.graph_offset;
    let visible_oids: Vec<git2::Oid> = app
        .active_branch_oids
        .iter()
        .skip(visible_start)
        .take(inner_height)
        .copied()
        .collect();

    let mut y = inner.y;
    for (local_idx, oid) in visible_oids.iter().enumerate() {
        let global_idx = local_idx + visible_start;
        let is_selected = global_idx == app.graph_selected && focused;

        let commit = match app.repo_data.commits.get(oid) {
            Some(c) => c,
            None => continue,
        };

        let graph_prefix = app
            .graph_index
            .get(oid)
            .and_then(|&i| app.graph_rows.get(i))
            .map(render_graph_prefix)
            .unwrap_or_else(|| "● ".to_string());

        let row_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };

        render_commit_row(
            frame,
            row_area,
            CommitRowData {
                commit,
                graph_prefix: &graph_prefix,
                is_selected,
                oid_to_branches: &app.repo_data.oid_to_branches,
                max_width: inner.width as usize,
                now,
            },
        );

        y += 1;
        if y >= inner.y + inner.height {
            break;
        }
    }
}

struct CommitRowData<'a> {
    commit: &'a CommitInfo,
    graph_prefix: &'a str,
    is_selected: bool,
    oid_to_branches: &'a HashMap<git2::Oid, Vec<String>>,
    max_width: usize,
    now: i64,
}

fn render_commit_row(frame: &mut Frame, area: Rect, data: CommitRowData<'_>) {
    let CommitRowData {
        commit,
        graph_prefix,
        is_selected,
        oid_to_branches,
        max_width,
        now,
    } = data;

    let bg = if is_selected {
        Style::default().bg(COLOR_SELECTED_BG)
    } else {
        Style::default()
    };

    let mut spans: Vec<Span> = Vec::new();

    for ch in graph_prefix.chars() {
        let (c, style) = match ch {
            '●' => (ch, Style::default().fg(COLOR_GRAPH_NODE)),
            '│' | '├' | '╮' | '╭' | '╯' | '╰' | '─' => {
                (ch, Style::default().fg(COLOR_GRAPH_LINE))
            }
            _ => (ch, Style::default().fg(COLOR_DIM)),
        };
        spans.push(Span::styled(c.to_string(), style.patch(bg)));
    }

    if let Some(labels) = oid_to_branches.get(&commit.oid) {
        for label in labels.iter().take(2) {
            spans.push(Span::styled(
                format!("[{}] ", label),
                Style::default()
                    .fg(COLOR_TAG)
                    .add_modifier(Modifier::BOLD)
                    .patch(bg),
            ));
        }
    }

    spans.push(Span::styled(
        format!("{} ", commit.short_id),
        Style::default().fg(COLOR_DIM).patch(bg),
    ));

    let used: usize = spans
        .iter()
        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let remaining = max_width.saturating_sub(used + 8);
    let msg = truncate_str(&commit.message, remaining);
    spans.push(Span::styled(msg, Style::default().patch(bg)));

    let time_str = format_relative_time_with_now(commit.time, now);
    spans.push(Span::styled(
        format!(" {}", time_str),
        Style::default().fg(COLOR_DIM).patch(bg),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
}
