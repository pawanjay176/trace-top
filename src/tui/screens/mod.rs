mod aggregates;
mod footer;
mod trace_detail;
mod trace_list;

use ratatui::Frame;

use super::app::{AppState, Screen};

pub fn render(frame: &mut Frame, app: &mut AppState) {
    match app.screen {
        Screen::TraceList => trace_list::render(frame, app),
        Screen::TraceDetail => trace_detail::render(frame, app),
        Screen::Aggregates => aggregates::render(frame, app),
        Screen::AggregateSpans => aggregates::render_spans(frame, app),
    }
}
