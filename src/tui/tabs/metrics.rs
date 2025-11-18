use crate::monitor::collector::MonitorSnapshot;
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
    snapshot: &Option<MonitorSnapshot>,
    _scroll_offset: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)].as_ref())
        .split(area);

    // Server status
    render_server_status(f, chunks[0], snapshot);

    // Backend statistics
    render_backend_stats(f, chunks[1], snapshot);
}

fn render_server_status(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let status_text = if let Some(snap) = snapshot {
        let uptime_hours = snap.server_status.uptime_seconds / 3600;
        let uptime_mins = (snap.server_status.uptime_seconds % 3600) / 60;

        vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled("● Running", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  Uptime: {}h {}m", uptime_hours, uptime_mins),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Total Requests: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_number(snap.server_status.total_requests),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("  Rate: {:.1} req/s", snap.request_rate),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Active Connections: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    snap.server_status.active_connections.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Error Rate: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.2}%", snap.error_rate * 100.0),
                    if snap.error_rate > 0.01 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
        ]
    } else {
        vec![Line::from("Loading...")]
    };

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Server Status"));

    f.render_widget(paragraph, area);
}

fn render_backend_stats(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let items: Vec<ListItem> = if let Some(snap) = snapshot {
        snap.server_status
            .backends
            .iter()
            .map(|(name, stats)| {
                let error_rate = if stats.requests > 0 {
                    (stats.errors as f64 / stats.requests as f64) * 100.0
                } else {
                    0.0
                };

                let content = Line::from(vec![
                    Span::styled(
                        format!("{:12}", name),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("Reqs: {:>8}", format_number(stats.requests)),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("Errors: {:>6}", stats.errors),
                        if stats.errors > 0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::Green)
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("Avg: {:>6.1}ms", stats.avg_response_ms),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("Err Rate: {:>5.2}%", error_rate),
                        if error_rate > 1.0 {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::Green)
                        },
                    ),
                ]);

                ListItem::new(content)
            })
            .collect()
    } else {
        vec![ListItem::new("No data")]
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Backend Statistics"));

    f.render_widget(list, area);
}

fn format_number(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}
