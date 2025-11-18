use crate::monitor::collector::MonitorSnapshot;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
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
        .constraints([
            Constraint::Length(10), // Server info
            Constraint::Length(6),  // Request metrics
            Constraint::Min(0),     // Backend summary
        ])
        .split(area);

    render_server_info(f, chunks[0], snapshot);
    render_request_metrics(f, chunks[1], snapshot);
    render_backend_summary(f, chunks[2], snapshot);
}

fn render_server_info(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let content = if let Some(snap) = snapshot {
        let uptime_days = snap.server_status.uptime_seconds / 86400;
        let uptime_hours = (snap.server_status.uptime_seconds % 86400) / 3600;
        let uptime_mins = (snap.server_status.uptime_seconds % 3600) / 60;
        let uptime_secs = snap.server_status.uptime_seconds % 60;

        vec![
            Line::from(vec![
                Span::styled("Server: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "fe-php v0.1.0",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "● Running",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    if uptime_days > 0 {
                        format!("{}d {}h {}m {}s", uptime_days, uptime_hours, uptime_mins, uptime_secs)
                    } else if uptime_hours > 0 {
                        format!("{}h {}m {}s", uptime_hours, uptime_mins, uptime_secs)
                    } else {
                        format!("{}m {}s", uptime_mins, uptime_secs)
                    },
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Active Connections: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    snap.server_status.active_connections.to_string(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Total Backends: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    snap.server_status.backends.len().to_string(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Loading server information...",
                Style::default().fg(Color::Yellow),
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Server Overview"));

    f.render_widget(paragraph, area);
}

fn render_request_metrics(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    if let Some(snap) = snapshot {
        // Request rate
        let rate_text = vec![
            Line::from(vec![
                Span::styled("Requests/sec: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.2}", snap.request_rate),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Total Requests: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_number(snap.server_status.total_requests),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let rate_widget = Paragraph::new(rate_text)
            .block(Block::default().borders(Borders::ALL).title("Request Metrics"));

        f.render_widget(rate_widget, chunks[0]);

        // Error rate with gauge
        let error_percent = (snap.error_rate * 100.0).min(100.0);
        let error_color = if error_percent > 5.0 {
            Color::Red
        } else if error_percent > 1.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Error Rate"))
            .gauge_style(Style::default().fg(error_color).bg(Color::Black))
            .percent(error_percent as u16)
            .label(format!("{:.2}%", error_percent));

        f.render_widget(gauge, chunks[1]);
    } else {
        let loading1 = Paragraph::new("Loading...")
            .block(Block::default().borders(Borders::ALL).title("Server Info"));
        let loading2 = Paragraph::new("Loading...")
            .block(Block::default().borders(Borders::ALL).title("Metrics"));
        f.render_widget(loading1, chunks[0]);
        f.render_widget(loading2, chunks[1]);
    }
}

fn render_backend_summary(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let content = if let Some(snap) = snapshot {
        let mut lines = vec![
            Line::from(Span::styled(
                "Backend Status:",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (name, stats) in &snap.server_status.backends {
            let status_symbol = if stats.errors == 0 {
                Span::styled("●", Style::default().fg(Color::Green))
            } else if (stats.errors as f64 / stats.requests as f64) > 0.05 {
                Span::styled("●", Style::default().fg(Color::Red))
            } else {
                Span::styled("●", Style::default().fg(Color::Yellow))
            };

            lines.push(Line::from(vec![
                status_symbol,
                Span::raw(" "),
                Span::styled(
                    format!("{:20}", name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>8} reqs", format_number(stats.requests)),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{:>6.1}ms avg", stats.avg_response_ms),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }

        if snap.server_status.backends.is_empty() {
            lines.push(Line::from(Span::styled(
                "No backends configured",
                Style::default().fg(Color::Gray),
            )));
        }

        lines
    } else {
        vec![Line::from("Loading backend information...")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Backends"));

    f.render_widget(paragraph, area);
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
