use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use std::time::Duration;

use super::footer;
use crate::tui::app::{AppState, Screen};

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

    let search_style = if app.is_search_editing(Screen::TraceDetail) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let filter_style = if app.is_filter_editing(Screen::TraceDetail) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Trace Detail",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  search="),
        Span::styled(app.search_label(Screen::TraceDetail), search_style),
        Span::raw("  filter="),
        Span::styled(app.filter_label(Screen::TraceDetail), filter_style),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, layout[0]);

    render_spans(frame, app, layout[1]);

    footer::render(
        frame,
        app,
        layout[2],
        Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" move span  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" attributes  "),
            Span::styled("b/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" back  "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" aggregates  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" refresh  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("f", Style::default().fg(Color::Yellow)),
            Span::raw(" filter  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]),
    );
}

fn render_spans(frame: &mut Frame, app: &mut AppState, area: ratatui::layout::Rect) {
    let visible_rows = app.trace_detail_visible_rows();
    let mut selected_rendered_index = None;
    let mut selected_is_expanded = false;
    let rows = if app.selected_trace.is_none() {
        vec![Row::new(vec![Cell::from("No trace selected or loaded.")])]
    } else if visible_rows.is_empty() {
        vec![Row::new(vec![Cell::from("No spans match search.")])]
    } else {
        let mut rows = Vec::new();
        for (source_index, span) in &visible_rows {
            if *source_index == app.selected_span_index {
                selected_rendered_index = Some(rows.len());
                selected_is_expanded = app.span_attributes_expanded(span);
            }

            rows.push(Row::new(vec![
                Cell::from(format!("{}{}", "  ".repeat(span.depth), span.name)),
                Cell::from(format_duration_ms(Duration::from_nanos(
                    span.end_unix_nano.saturating_sub(span.start_unix_nano),
                ))),
            ]));

            if app.span_attributes_expanded(span) {
                rows.extend(attribute_rows(span.depth, &span.attributes));
            }
        }
        rows
    };

    let table = Table::new(rows, [Constraint::Min(20), Constraint::Length(12)])
        .header(Row::new(["span", "duration"]).style(Style::default().fg(Color::DarkGray)))
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
    if !visible_rows.is_empty() {
        state.select(selected_rendered_index);
        if selected_is_expanded {
            *state.offset_mut() = selected_rendered_index.unwrap_or(0).saturating_sub(2);
        }
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn format_duration_ms(duration: Duration) -> String {
    format!("{:.2}ms", duration.as_nanos() as f64 / 1_000_000.0)
}

fn attribute_rows(depth: usize, attributes: &[(String, String)]) -> Vec<Row<'static>> {
    let indent = "  ".repeat(depth + 1);
    if attributes.is_empty() {
        return vec![attribute_row(format!("{indent}attributes: <none>"))];
    }

    attributes
        .iter()
        .map(|(key, value)| attribute_row(format!("{indent}{key}: {value}")))
        .collect()
}

fn attribute_row(line: String) -> Row<'static> {
    Row::new(vec![Cell::from(line), Cell::from("")]).style(Style::default().fg(Color::DarkGray))
}
