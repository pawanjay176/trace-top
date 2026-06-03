use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};

use crate::tui::app::AppState;

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);

    frame.render_widget(Clear, area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Trace Detail",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  trace_id="),
        Span::raw(app.selected_trace_id_text()),
        Span::raw("  snapshot_trace_id="),
        Span::raw(
            app.selected_trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("<none>"),
        ),
        Span::raw("  version="),
        Span::raw(
            app.selected_trace
                .as_ref()
                .map(|trace| trace.version.to_string())
                .unwrap_or_else(|| "<none>".into()),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, layout[0]);

    render_spans(frame, app, layout[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" move span  "),
        Span::styled("b/Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back  "),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(" aggregates  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, layout[2]);
}

fn render_spans(frame: &mut Frame, app: &mut AppState, area: ratatui::layout::Rect) {
    let rows = app
        .selected_trace
        .as_ref()
        .map(|trace| {
            trace
                .rows
                .iter()
                .map(|span| {
                    Row::new(vec![
                        Cell::from(format!("{}{}", "  ".repeat(span.depth), span.name)),
                        Cell::from(format_duration(
                            span.end_unix_nano.saturating_sub(span.start_unix_nano),
                        )),
                        Cell::from(short_span_id(&span.span_id)),
                        Cell::from(
                            span.parent_span_id
                                .as_deref()
                                .map(short_span_id)
                                .unwrap_or_else(|| "-".into()),
                        ),
                    ])
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![Row::new(vec![Cell::from("No trace selected or loaded.")])]);

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(65),
            Constraint::Length(14),
            Constraint::Length(18),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(["span", "duration", "span id", "parent"])
            .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .title(" Waterfall Scaffold ")
            .borders(Borders::ALL),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(26, 34, 48))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    let mut state = TableState::default();
    if app
        .selected_trace
        .as_ref()
        .is_some_and(|trace| !trace.rows.is_empty())
    {
        state.select(Some(app.selected_span_index));
    }
    frame.render_stateful_widget(table, area, &mut state);

    if let Some(details) = app
        .selected_trace
        .as_ref()
        .and_then(|trace| trace.selected_span.as_ref())
    {
        let _ = (
            &details.span_id,
            &details.name,
            details.start_unix_nano,
            details.end_unix_nano,
            details.attributes.len(),
        );
    }
}

fn short_span_id(span_id: &str) -> String {
    span_id.chars().take(16).collect()
}

fn format_duration(duration_nano: u64) -> String {
    if duration_nano >= 1_000_000 {
        format!("{:.2}ms", duration_nano as f64 / 1_000_000.0)
    } else if duration_nano >= 1_000 {
        format!("{:.2}us", duration_nano as f64 / 1_000.0)
    } else {
        format!("{duration_nano}ns")
    }
}
