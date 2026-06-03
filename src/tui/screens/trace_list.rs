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
            Constraint::Length(5),
        ])
        .split(area);

    frame.render_widget(Clear, area);
    render_header(frame, app, layout[0]);
    render_table(frame, app, layout[1]);
    render_footer(frame, app, layout[2]);
}

fn render_header(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let search = app.trace_list_query.search.as_deref().unwrap_or("<none>");
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "trace-tui",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  recent traces"),
        Span::raw("  search="),
        Span::styled(search, Style::default().fg(Color::Yellow)),
        Span::raw("  limit="),
        Span::raw(app.trace_list_query.limit.to_string()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, app: &mut AppState, area: ratatui::layout::Rect) {
    let rows = if app.trace_list.rows.is_empty() {
        vec![Row::new(vec![Cell::from(
            "No traces loaded. Waiting for a store change notification.",
        )])]
    } else {
        app.trace_list
            .rows
            .iter()
            .map(|trace| {
                Row::new(vec![
                    Cell::from(short_trace_id(&trace.trace_id)),
                    Cell::from(
                        trace
                            .root_name
                            .clone()
                            .unwrap_or_else(|| "<unknown>".into()),
                    ),
                    Cell::from(trace.start_unix_nano.to_string()),
                    Cell::from(format_duration(trace.duration_nano)),
                    Cell::from(trace.span_count.to_string()),
                    Cell::from(trace.error_count.to_string()),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Percentage(35),
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new([
            "trace id",
            "root span",
            "start ns",
            "duration",
            "spans",
            "errors",
        ])
        .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .title(" Recent Traces ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(26, 34, 48))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    let mut state = TableState::default();
    if !app.trace_list.rows.is_empty() {
        state.select(Some(app.selected_trace_index));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_footer(frame: &mut Frame, app: &AppState, area: ratatui::layout::Rect) {
    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" move  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" open  "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" aggregates  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" reset search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]),
        Line::from(vec![
            Span::styled("selected trace_id:", Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::raw(app.selected_trace_id_text()),
            Span::raw("  version="),
            Span::raw(app.store_version_seen.to_string()),
            Span::raw("  snapshot_version="),
            Span::raw(app.trace_list.version.to_string()),
            Span::raw("  traces="),
            Span::raw(app.trace_list.total_traces.to_string()),
            Span::raw("  spans="),
            Span::raw(app.trace_list.total_spans.to_string()),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

fn short_trace_id(trace_id: &str) -> String {
    trace_id.chars().take(16).collect()
}

fn format_duration(duration_nano: u64) -> String {
    if duration_nano >= 1_000_000_000 {
        format!("{:.2}s", duration_nano as f64 / 1_000_000_000.0)
    } else if duration_nano >= 1_000_000 {
        format!("{:.2}ms", duration_nano as f64 / 1_000_000.0)
    } else if duration_nano >= 1_000 {
        format!("{:.2}us", duration_nano as f64 / 1_000.0)
    } else {
        format!("{duration_nano}ns")
    }
}
