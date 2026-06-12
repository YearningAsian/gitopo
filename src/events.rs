use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_ms),
        }
    }

    pub fn next(&self) -> Result<AppEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                // Only react to key presses: on Windows, crossterm also delivers
                // Release (and Repeat) events, which would double every keystroke.
                Event::Key(key) if key.kind == KeyEventKind::Press => Ok(AppEvent::Key(key)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}

/// Normalised key action, decoupled from raw crossterm events
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
    SelectBranch,
    ToggleAll,
    Search,
    SearchNext,
    SearchPrev,
    #[allow(dead_code)]
    ClearSearch,
    Help,
    Refresh,
    Escape,
    Char(char),
    Backspace,
    #[allow(dead_code)]
    Enter,
}

pub fn key_to_action(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => Some(Action::MoveUp),
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => Some(Action::MoveDown),
        (KeyCode::PageUp, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => Some(Action::PageUp),
        (KeyCode::PageDown, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
            Some(Action::PageDown)
        }
        (KeyCode::Home, _) | (KeyCode::Char('g'), _) => Some(Action::Top),
        (KeyCode::End, _) | (KeyCode::Char('G'), _) => Some(Action::Bottom),
        (KeyCode::Enter, _) | (KeyCode::Char('l'), _) | (KeyCode::Right, _) => {
            Some(Action::SelectBranch)
        }
        (KeyCode::Char('a'), _) => Some(Action::ToggleAll),
        (KeyCode::Char('/'), _) => Some(Action::Search),
        (KeyCode::Char('n'), _) => Some(Action::SearchNext),
        (KeyCode::Char('N'), _) => Some(Action::SearchPrev),
        (KeyCode::Char('r'), _) => Some(Action::Refresh),
        (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => Some(Action::Help),
        (KeyCode::Esc, _) | (KeyCode::Char('h'), _) | (KeyCode::Left, _) => Some(Action::Escape),
        (KeyCode::Backspace, _) | (KeyCode::Delete, _) => Some(Action::Backspace),
        (KeyCode::Char(c), _) => Some(Action::Char(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn quit_bindings() {
        assert_eq!(key_to_action(key(KeyCode::Char('q'))), Some(Action::Quit));
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn navigation_bindings() {
        assert_eq!(
            key_to_action(key(KeyCode::Char('j'))),
            Some(Action::MoveDown)
        );
        assert_eq!(key_to_action(key(KeyCode::Up)), Some(Action::MoveUp));
        assert_eq!(key_to_action(key(KeyCode::Char('g'))), Some(Action::Top));
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Some(Action::Bottom)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
            Some(Action::PageDown)
        );
    }

    #[test]
    fn mode_bindings() {
        assert_eq!(key_to_action(key(KeyCode::Char('/'))), Some(Action::Search));
        assert_eq!(
            key_to_action(key(KeyCode::Char('a'))),
            Some(Action::ToggleAll)
        );
        assert_eq!(key_to_action(key(KeyCode::Char('?'))), Some(Action::Help));
        assert_eq!(key_to_action(key(KeyCode::Esc)), Some(Action::Escape));
        assert_eq!(key_to_action(key(KeyCode::F(5))), None);
    }
}
