use crate::monitor::analyzer::LogAnalysisResult;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(
    f: &mut Frame,
    area: Rect,
    analysis: &Option<LogAnalysisResult>,
    _scroll_offset: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(4),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ]
            .as_ref(),
        )
        .split(area);

    // Summary
    render_summary(f, chunks[0], analysis);

    // Top endpoints
    render_top_endpoints(f, chunks[1], analysis);

    // Slow requests
    render_slow_requests(f, chunks[2], analysis);

    // Suspicious activity
    render_suspicious_activity(f, chunks[3], analysis);
}

fn render_summary(
    f: &mut Frame,
    area: Rect,
    analysis: &Option<LogAnalysisResult>,
) {
    let summary_text = if let Some(result) = analysis {
        let error_rate = if result.total_requests > 0 {
            (result.error_count as f64 / result.total_requests as f64) * 100.0
        } else {
            0.0
        };

        vec![
            Line::from(vec![
                Span::styled("Total Requests: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    result.total_requests.to_string(),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("  Errors: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    result.error_count.to_string(),
                    if result.error_count > 0 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
                Span::styled(
                    format!("  ({:.2}%)", error_rate),
                    Style::default().fg(Color::White),
                ),
            ]),
        ]
    } else {
        vec![Line::from("No analysis data")]
    };

    let paragraph = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title("Summary"));

    f.render_widget(paragraph, area);
}

fn render_top_endpoints(
    f: &mut Frame,
    area: Rect,
    analysis: &Option<LogAnalysisResult>,
) {
    let items: Vec<ListItem> = if let Some(result) = analysis {
        result
            .top_endpoints
            .iter()
            .map(|endpoint| {
                let content = Line::from(vec![
                    Span::styled(
                        format!("{:6}", endpoint.count),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:>6.1}ms", endpoint.avg_duration_ms),
                        if endpoint.avg_duration_ms > 100.0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::Green)
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("err:{:>4}", endpoint.error_count),
                        if endpoint.error_count > 0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::Green)
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(&endpoint.path, Style::default().fg(Color::White)),
                ]);

                ListItem::new(content)
            })
            .collect()
    } else {
        vec![ListItem::new("No data")]
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Top Endpoints (by count)"),
    );

    f.render_widget(list, area);
}

fn render_slow_requests(
    f: &mut Frame,
    area: Rect,
    analysis: &Option<LogAnalysisResult>,
) {
    let items: Vec<ListItem> = if let Some(result) = analysis {
        result
            .slow_requests
            .iter()
            .map(|req| {
                let content = Line::from(vec![
                    Span::styled(
                        format!("{:>6}ms", req.duration_ms),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:6}", req.method),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::styled(&req.uri, Style::default().fg(Color::White)),
                ]);

                ListItem::new(content)
            })
            .collect()
    } else {
        vec![ListItem::new("No slow requests")]
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Slow Requests (>100ms)"),
    );

    f.render_widget(list, area);
}

fn render_suspicious_activity(
    f: &mut Frame,
    area: Rect,
    analysis: &Option<LogAnalysisResult>,
) {
    let items: Vec<ListItem> = if let Some(result) = analysis {
        result
            .suspicious_activity
            .iter()
            .map(|activity| {
                let content = Line::from(vec![
                    Span::styled(
                        &activity.ip_address,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("[{}]", activity.event_type),
                        Style::default().fg(Color::Red),
                    ),
                    Span::raw("  "),
                    Span::styled(&activity.description, Style::default().fg(Color::White)),
                ]);

                ListItem::new(content)
            })
            .collect()
    } else {
        vec![ListItem::new("No suspicious activity detected")]
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Suspicious Activity"),
    );

    f.render_widget(list, area);
}
