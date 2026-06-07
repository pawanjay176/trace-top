use crossterm::event::{KeyCode, KeyEvent};

use super::app::{Action, AppState, Screen};

pub fn map_key(key: KeyEvent, app: &AppState) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => match app.screen {
            Screen::AggregateSpans => Action::ShowAggregates,
            _ => Action::ShowTraceList,
        },
        KeyCode::Enter => match app.screen {
            Screen::TraceList => Action::ShowTraceDetail,
            Screen::Aggregates => Action::ShowAggregateSpans,
            Screen::AggregateSpans => Action::OpenAggregateSpanTrace,
            _ => Action::Noop,
        },
        KeyCode::Char('a') => Action::ShowAggregates,
        KeyCode::Char('b') => match app.screen {
            Screen::AggregateSpans => Action::ShowAggregates,
            _ => Action::ShowTraceList,
        },
        KeyCode::Char('r') => Action::RefreshCurrentScreen,
        KeyCode::Down | KeyCode::Char('j') => match app.screen {
            Screen::TraceList => Action::MoveSelectionDown,
            Screen::TraceDetail => Action::MoveSpanDown,
            Screen::Aggregates | Screen::AggregateSpans => Action::MoveSelectionDown,
        },
        KeyCode::Up | KeyCode::Char('k') => match app.screen {
            Screen::TraceList => Action::MoveSelectionUp,
            Screen::TraceDetail => Action::MoveSpanUp,
            Screen::Aggregates | Screen::AggregateSpans => Action::MoveSelectionUp,
        },
        _ => Action::Noop,
    }
}
