use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Focus};
use crate::git::{format_relative_time_with_now, BranchInfo};

use super::theme::{
    truncate_str, COLOR_AHEAD, COLOR_BEHIND, COLOR_BORDER_FOCUSED, COLOR_BORDER_UNFOCUSED,
    COLOR_DIM, COLOR_HEAD, COLOR_LOCAL, COLOR_REMOTE, COLOR_SELECTED_BG, COLOR_TITLE,
};

pub fn render_branch_list(frame: &mut Frame, app: &mut App, area: Rect, now: i64) {
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
            display_name,
            Style::default()
                .fg(name_color)
                .add_modifier(if branch.is_head {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ];

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

    spans.push(Span::styled(
        format!(" {}", time_str),
        Style::default().fg(COLOR_DIM),
    ));

    let style = if is_selected {
        Style::default().bg(COLOR_SELECTED_BG)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}
