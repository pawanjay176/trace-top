use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{Action, AppState, Screen};

pub fn map_key(key: KeyEvent, app: &AppState) -> Action {
    if app.search.active {
        return match key.code {
            KeyCode::Esc => Action::CancelSearch,
            KeyCode::Enter => Action::SubmitSearch,
            KeyCode::Backspace => Action::SearchBackspace,
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::SearchPush(ch)
            }
            _ => Action::Noop,
        };
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => {
            if app.has_current_screen_search() {
                Action::ClearSearch
            } else {
                match app.screen {
                    Screen::AggregateSpans => Action::ShowAggregates,
                    _ => Action::ShowTraceList,
                }
            }
        }
        KeyCode::Enter => match app.screen {
            Screen::TraceList => Action::ShowTraceDetail,
            Screen::Aggregates => Action::ShowAggregateSpans,
            Screen::AggregateSpans => Action::OpenAggregateSpanTrace,
            _ => Action::Noop,
        },
        KeyCode::Char('a') => Action::ShowAggregates,
        KeyCode::Char('/') => Action::StartSearch,
        KeyCode::Char('f') => Action::StartFilter,
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
