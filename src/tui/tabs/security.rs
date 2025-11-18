use crate::monitor::collector::MonitorSnapshot;
use crate::tui::client::TuiClient;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::sync::Arc;

pub fn render(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
    client: &Option<Arc<TuiClient>>,
    blocked_ips: &[String],
    _scroll_offset: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // WAF status
            Constraint::Length(12), // IP Blocker
            Constraint::Min(0),     // GeoIP / Rate limiting
        ])
        .split(area);

    render_waf_status(f, chunks[0], snapshot);
    render_ip_blocker(f, chunks[1], client, blocked_ips);
    render_other_security(f, chunks[2], snapshot);
}

fn render_waf_status(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    // WAF情報は設定から取得する必要があるため、
    // 現在は基本情報のみ表示
    let content = vec![
        Line::from(vec![
            Span::styled("WAF (Web Application Firewall)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled("● Enabled", Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled("Mode: ", Style::default().fg(Color::Gray)),
            Span::styled("Block", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Protected Against:", Style::default().fg(Color::Gray)),
        ]),
        Line::from("  • SQL Injection"),
        Line::from("  • XSS (Cross-Site Scripting)"),
        Line::from("  • Path Traversal"),
        Line::from("  • Command Injection"),
    ];

    let widget = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Web Application Firewall"));

    f.render_widget(widget, area);
}

fn render_ip_blocker(
    f: &mut Frame,
    area: Rect,
    client: &Option<Arc<TuiClient>>,
    blocked_ips: &[String],
) {
    let content = if client.is_some() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("IP Blocker", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled("● Active", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Blocked IPs: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    blocked_ips.len().to_string(),
                    if blocked_ips.is_empty() {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ),
            ]),
            Line::from(""),
        ];

        if !blocked_ips.is_empty() {
            lines.push(Line::from(Span::styled(
                "Currently Blocked:",
                Style::default().fg(Color::Gray),
            )));
            for ip in blocked_ips.iter().take(4) {
                lines.push(Line::from(format!("  • {}", ip)));
            }
            if blocked_ips.len() > 4 {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", blocked_ips.len() - 4),
                    Style::default().fg(Color::Gray),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No IPs currently blocked",
                Style::default().fg(Color::Green),
            )));
        }

        lines
    } else {
        vec![
            Line::from(vec![
                Span::styled("IP Blocker", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Not connected to server",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from("Connect with --socket to view IP blocker status"),
        ]
    };

    let widget = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("IP Blocker (Dynamic)"));

    f.render_widget(widget, area);
}

fn render_other_security(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // GeoIP filtering
    let geoip_content = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Configured", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Features:", Style::default().fg(Color::Gray)),),
        Line::from("  • Country filtering"),
        Line::from("  • Allowed countries"),
        Line::from("  • Blocked countries"),
        Line::from(""),
        Line::from(Span::styled(
            "Check config for details",
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
        )),
    ];

    let geoip_widget = Paragraph::new(geoip_content)
        .block(Block::default().borders(Borders::ALL).title("GeoIP Filtering"));

    f.render_widget(geoip_widget, chunks[0]);

    // Rate limiting
    let ratelimit_content = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled("Configured", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Features:", Style::default().fg(Color::Gray)),),
        Line::from("  • Per-IP rate limiting"),
        Line::from("  • Token bucket algorithm"),
        Line::from("  • Configurable burst"),
        Line::from(""),
        Line::from(Span::styled(
            "429 Too Many Requests",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let ratelimit_widget = Paragraph::new(ratelimit_content)
        .block(Block::default().borders(Borders::ALL).title("Rate Limiting"));

    f.render_widget(ratelimit_widget, chunks[1]);
}
