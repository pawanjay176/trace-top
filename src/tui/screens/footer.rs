use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::AppState;

pub fn render(frame: &mut Frame, app: &AppState, area: Rect, key_line: Line<'static>) {
    let footer = Paragraph::new(vec![key_line, status_line(app)]).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

fn status_line(app: &AppState) -> Line<'static> {
    Line::from(vec![
        Span::styled("selected trace_id:", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw(app.selected_trace_id_text().to_owned()),
        Span::raw("  traces="),
        Span::raw(app.trace_list.total_traces.to_string()),
        Span::raw("  spans="),
        Span::raw(app.trace_list.total_spans.to_string()),
        Span::raw("  store="),
        Span::raw(format_store_gb(app.trace_list.estimated_store_bytes)),
    ])
}

fn format_store_gb(bytes: usize) -> String {
    format!("{:.3}GB", bytes as f64 / 1_000_000_000.0)
}
