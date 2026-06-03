use crossterm::event::{KeyCode, KeyEvent};

use super::app::{Action, AppState, Screen};

pub fn map_key(key: KeyEvent, app: &AppState) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => Action::ShowTraceList,
        KeyCode::Enter => match app.screen {
            Screen::TraceList => Action::ShowTraceDetail,
            _ => Action::Noop,
        },
        KeyCode::Char('a') => Action::ShowAggregates,
        KeyCode::Char('b') => Action::ShowTraceList,
        KeyCode::Char('r') => Action::ClearTraceSearch,
        KeyCode::Down | KeyCode::Char('j') => match app.screen {
            Screen::TraceList => Action::MoveSelectionDown,
            Screen::TraceDetail => Action::MoveSpanDown,
            Screen::Aggregates => Action::Noop,
        },
        KeyCode::Up | KeyCode::Char('k') => match app.screen {
            Screen::TraceList => Action::MoveSelectionUp,
            Screen::TraceDetail => Action::MoveSpanUp,
            Screen::Aggregates => Action::Noop,
        },
        _ => Action::Noop,
    }
}
