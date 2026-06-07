use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};

use crate::tui::app::{AppState, Screen};

use jiff::{Timestamp, tz::TimeZone};

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

    let search_style = if app.is_search_editing(Screen::Aggregates) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Aggregates",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  group_by="),
        Span::raw(
            app.aggregate_query
                .group_by_attribute
                .as_deref()
                .unwrap_or("<none>"),
        ),
        Span::raw("  search="),
        Span::styled(app.search_label(Screen::Aggregates), search_style),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    let rows = if app.aggregate.rows.is_empty() {
        vec![Row::new(vec![Cell::from("No aggregate data loaded.")])]
    } else {
        app.aggregate
            .rows
            .iter()
            .map(|row| {
                Row::new(vec![
                    Cell::from(row.span_name.clone()),
                    Cell::from(row.group.clone().unwrap_or_else(|| "-".into())),
                    Cell::from(row.calls.to_string()),
                    Cell::from(format_duration(row.mean_nano)),
                    Cell::from(format_duration(row.p50_nano)),
                    Cell::from(format_duration(row.p95_nano)),
                    Cell::from(format_duration(row.max_nano)),
                    Cell::from(row.error_count.to_string()),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(20),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new([
            "span", "group", "calls", "mean", "p50", "p95", "max", "errors",
        ])
        .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .title(" Span Aggregates ")
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
    if !app.aggregate.rows.is_empty() {
        state.select(Some(app.selected_aggregate_index));
    }
    frame.render_stateful_widget(table, layout[1], &mut state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" move  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" spans  "),
        Span::styled("b/Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" refresh  "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" search  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, layout[2]);
}

pub fn render_spans(frame: &mut Frame, app: &mut AppState) {
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

    let query = app.aggregate_spans_query.as_ref();
    let search_style = if app.is_search_editing(Screen::AggregateSpans) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Aggregate Spans",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  span="),
        Span::raw(
            query
                .map(|query| query.span_name.as_str())
                .unwrap_or("<none>"),
        ),
        Span::raw("  group="),
        Span::raw(
            query
                .and_then(|query| query.group.as_deref())
                .unwrap_or("<none>"),
        ),
        Span::raw("  search="),
        Span::styled(app.search_label(Screen::AggregateSpans), search_style),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    let rows = if app.aggregate_spans.rows.is_empty() {
        vec![Row::new(vec![Cell::from(
            "No spans for selected aggregate.",
        )])]
    } else {
        app.aggregate_spans
            .rows
            .iter()
            .map(|row| {
                Row::new(vec![
                    Cell::from(row.span_name.clone()),
                    Cell::from(format_duration(row.duration_nano)),
                    Cell::from(format_start_time(row.start_unix_nano)),
                    Cell::from(short_trace_id(&row.trace_id)),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(45),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(18),
        ],
    )
    .header(
        Row::new(["span", "duration", "start", "trace"])
            .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .title(" Spans By Duration ")
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
    if !app.aggregate_spans.rows.is_empty() {
        state.select(Some(app.selected_aggregate_span_index));
    }
    frame.render_stateful_widget(table, layout[1], &mut state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" move  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" open trace  "),
        Span::styled("b/Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" aggregates  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" refresh  "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" search  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, layout[2]);
}

fn format_duration(duration_nano: u64) -> String {
    format!("{:.2}ms", duration_nano as f64 / 1_000_000.0)
}

fn format_start_time(unix_nano: u64) -> String {
    if unix_nano == 0 {
        return "-".into();
    }

    Timestamp::from_nanosecond(unix_nano as i128)
        .map(|timestamp| {
            timestamp
                .to_zoned(TimeZone::system())
                .strftime("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| "-".into())
}

fn short_trace_id(trace_id: &str) -> String {
    trace_id.chars().take(16).collect()
}
