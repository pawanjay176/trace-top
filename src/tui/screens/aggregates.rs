use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
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
        Span::raw(
            app.aggregate_query
                .span_name_search
                .as_deref()
                .unwrap_or("<none>"),
        ),
        Span::raw("  version="),
        Span::raw(app.aggregate.version.to_string()),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    let rows = if app.aggregate.rows.is_empty() {
        vec![Row::new(vec![Cell::from(
            "No aggregate data loaded. Waiting for store implementation.",
        )])]
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
    );
    frame.render_widget(table, layout[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("b/Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, layout[2]);
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
