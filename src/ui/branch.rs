use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Focus, SearchHighlight};
use crate::git::{format_relative_time_with_now, BranchInfo};

use super::theme::{truncate_str, Theme};

pub fn render_branch_list(frame: &mut Frame, app: &mut App, area: Rect, now: i64) {
    let inner_height = area.height.saturating_sub(2) as usize;
    app.scroll_branch_list(inner_height);

    let theme = app.theme;
    let focused = app.focus == Focus::BranchList;
    let border_color = if focused {
        theme.border_focused
    } else {
        theme.border_unfocused
    };

    let branch_count = app.repo_data.branches.len();
    let title = format!(" Branches ({}) ", branch_count);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.title)
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
            let highlight = app.search_highlight(i);
            build_branch_list_item(
                branch,
                is_selected,
                highlight,
                area.width as usize,
                now,
                &theme,
            )
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
    highlight: Option<SearchHighlight>,
    width: usize,
    now: i64,
    theme: &Theme,
) -> ListItem<'a> {
    let name_color = match highlight {
        Some(_) => theme.search_match,
        None if branch.is_head => theme.head,
        None if branch.is_remote => theme.remote,
        None => theme.local,
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

    // Underline search matches so they remain distinguishable even when colour
    // is disabled (--no-color) or the row also carries a selection background.
    let mut name_modifiers = if branch.is_head {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    if highlight.is_some() {
        name_modifiers |= Modifier::UNDERLINED;
    }

    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(name_color)),
        Span::styled(
            display_name,
            Style::default().fg(name_color).add_modifier(name_modifiers),
        ),
    ];

    if let Some((ahead, behind)) = branch.ahead_behind {
        if ahead > 0 || behind > 0 {
            spans.push(Span::raw(" "));
            if ahead > 0 {
                spans.push(Span::styled(
                    format!("↑{}", ahead),
                    Style::default().fg(theme.ahead),
                ));
            }
            if behind > 0 {
                spans.push(Span::styled(
                    format!("↓{}", behind),
                    Style::default().fg(theme.behind),
                ));
            }
        }
    }

    spans.push(Span::styled(
        format!(" {}", time_str),
        Style::default().fg(theme.dim),
    ));

    // Selection background wins; otherwise the active search match gets its own
    // tint so the n/N cursor is obvious among the dimmer matches.
    let style = if is_selected {
        Style::default().bg(theme.selected_bg)
    } else if highlight == Some(SearchHighlight::Active) {
        Style::default().bg(theme.search_active_bg)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}
