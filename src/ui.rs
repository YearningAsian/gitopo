use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{now_timestamp, App, Focus};
use crate::git::{format_relative_time_with_now, BranchInfo, CommitInfo};
use crate::graph::render_graph_prefix;

// Colour palette
const COLOR_HEAD: Color = Color::Rgb(100, 220, 100);
const COLOR_LOCAL: Color = Color::Rgb(130, 180, 255);
const COLOR_REMOTE: Color = Color::Rgb(255, 180, 80);
const COLOR_SELECTED_BG: Color = Color::Rgb(40, 55, 75);
const COLOR_DIM: Color = Color::Rgb(120, 120, 130);
const COLOR_GRAPH_NODE: Color = Color::Rgb(180, 140, 255);
const COLOR_GRAPH_LINE: Color = Color::Rgb(80, 80, 100);
const COLOR_TAG: Color = Color::Rgb(255, 220, 80);
const COLOR_AHEAD: Color = Color::Rgb(100, 220, 180);
const COLOR_BEHIND: Color = Color::Rgb(255, 120, 100);
const COLOR_BORDER_FOCUSED: Color = Color::Rgb(100, 160, 255);
const COLOR_BORDER_UNFOCUSED: Color = Color::Rgb(60, 60, 80);
const COLOR_TITLE: Color = Color::Rgb(200, 200, 220);

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let now = now_timestamp();

    if app.focus == Focus::Help {
        render_help(frame, area);
        return;
    }

    // Top: repo path bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_title_bar(frame, app, chunks[0]);

    let main = chunks[1];
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main);

    let left = main_chunks[0];
    let right = main_chunks[1];

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(right);

    render_branch_list(frame, app, left, now);
    render_commit_graph(frame, app, right_chunks[0], now);
    render_detail_pane(frame, app, right_chunks[1], now);
    render_status_bar(frame, app, chunks[2]);

    if app.search_mode {
        render_search_overlay(frame, app, area);
    }
}

fn render_title_bar(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_branch_list(frame: &mut Frame, app: &mut App, area: Rect, now: i64) {
    let inner_height = area.height.saturating_sub(2) as usize;
    app.scroll_branch_list(inner_height);

    let focused = app.focus == Focus::BranchList;
    let border_color = if focused {
        COLOR_BORDER_FOCUSED
    } else {
        COLOR_BORDER_UNFOCUSED
    };

    let branch_count = app.repo_data.branches.len();
    let title = format!(" Branches ({}) ", branch_count);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(COLOR_TITLE)
                .add_modifier(Modifier::BOLD),
        ));

    let visible_start = app.branch_offset;
    let visible_end = (visible_start + inner_height).min(branch_count);

    let items: Vec<ListItem> = app
        .repo_data
        .branches
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_end - visible_start)
        .map(|(i, branch)| {
            let is_selected = i == app.branch_selected;
            build_branch_list_item(branch, is_selected, area.width as usize, now)
        })
        .collect();

    let mut state = ListState::default();
    if app.branch_selected >= visible_start {
        state.select(Some(app.branch_selected - visible_start));
    }

    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut state);
}

fn build_branch_list_item<'a>(
    branch: &'a BranchInfo,
    is_selected: bool,
    width: usize,
    now: i64,
) -> ListItem<'a> {
    let name_color = if branch.is_head {
        COLOR_HEAD
    } else if branch.is_remote {
        COLOR_REMOTE
    } else {
        COLOR_LOCAL
    };

    let prefix = if branch.is_head {
        "* "
    } else if branch.is_remote {
        "↑ "
    } else {
        "  "
    };

    let max_name = width.saturating_sub(12);
    let display_name = truncate_str(&branch.name, max_name);

    let time_str = format_relative_time_with_now(branch.tip_time, now);

    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(name_color)),
        Span::styled(
            display_name.clone(),
            Style::default()
                .fg(name_color)
                .add_modifier(if branch.is_head {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ];

    // Ahead/behind indicator
    if let Some((ahead, behind)) = branch.ahead_behind {
        if ahead > 0 || behind > 0 {
            spans.push(Span::raw(" "));
            if ahead > 0 {
                spans.push(Span::styled(
                    format!("↑{}", ahead),
                    Style::default().fg(COLOR_AHEAD),
                ));
            }
            if behind > 0 {
                spans.push(Span::styled(
                    format!("↓{}", behind),
                    Style::default().fg(COLOR_BEHIND),
                ));
            }
        }
    }

    // Right-align timestamp
    let time_span = Span::styled(format!(" {}", time_str), Style::default().fg(COLOR_DIM));
    spans.push(time_span);

    let style = if is_selected {
        Style::default().bg(COLOR_SELECTED_BG)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}

fn render_commit_graph(frame: &mut Frame, app: &mut App, area: Rect, now: i64) {
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

        let graph_row = app
            .graph_index
            .get(oid)
            .and_then(|&i| app.graph_rows.get(i));
        let graph_prefix = graph_row
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
            commit,
            &graph_prefix,
            is_selected,
            &app.repo_data.oid_to_branches,
            inner.width as usize,
            now,
        );

        y += 1;
        if y >= inner.y + inner.height {
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_commit_row(
    frame: &mut Frame,
    area: Rect,
    commit: &CommitInfo,
    graph_prefix: &str,
    is_selected: bool,
    oid_to_branches: &std::collections::HashMap<git2::Oid, Vec<String>>,
    max_width: usize,
    now: i64,
) {
    let bg = if is_selected {
        Style::default().bg(COLOR_SELECTED_BG)
    } else {
        Style::default()
    };

    // Graph glyph
    let mut spans: Vec<Span> = Vec::new();

    // Color the graph prefix chars
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

    // Branch labels
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

    // Short hash
    spans.push(Span::styled(
        format!("{} ", commit.short_id),
        Style::default().fg(COLOR_DIM).patch(bg),
    ));

    // Message
    let used: usize = spans
        .iter()
        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let remaining = max_width.saturating_sub(used + 8); // reserve for time
    let msg = truncate_str(&commit.message, remaining);
    spans.push(Span::styled(msg, Style::default().patch(bg)));

    let time_str = format_relative_time_with_now(commit.time, now);
    spans.push(Span::styled(
        format!(" {}", time_str),
        Style::default().fg(COLOR_DIM).patch(bg),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)).style(bg), area);
}

fn render_detail_pane(frame: &mut Frame, app: &App, area: Rect, now: i64) {
    let border_color = COLOR_BORDER_UNFOCUSED;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
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

    // Show branch labels if any
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

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
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

    let para = Paragraph::new(msg).style(Style::default().fg(COLOR_DIM).bg(Color::Rgb(20, 22, 30)));
    frame.render_widget(para, area);
}

fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    const POPUP_HEIGHT: u16 = 3;
    if area.height < POPUP_HEIGHT + 1 || area.width < 8 {
        return; // terminal too small for the overlay
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

    let para = Paragraph::new(text).block(block);
    frame.render_widget(para, popup_area);
}

fn render_help(frame: &mut Frame, area: Rect) {
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

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, popup_area);
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

// ─── helpers ─────────────────────────────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        chars.iter().collect()
    } else {
        let mut out: String = chars.iter().take(max - 1).collect();
        out.push('…');
        out
    }
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.len() <= max {
        return path.to_owned();
    }
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.len() <= 2 {
        return truncate_str(path, max);
    }
    let short = format!("…/{}", parts[parts.len() - 1]);
    if short.len() <= max {
        short
    } else {
        truncate_str(path, max)
    }
}
