use crate::monitor::collector::MonitorSnapshot;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(
    f: &mut Frame,
    area: Rect,
    snapshot: &Option<MonitorSnapshot>,
    scroll_offset: usize,
) {
    if let Some(snap) = snapshot {
        if snap.server_status.backends.is_empty() {
            render_no_backends(f, area);
            return;
        }

        // Calculate items per backend
        let backend_count = snap.server_status.backends.len();
        let height_per_backend = if backend_count > 0 {
            (area.height as usize - 2) / backend_count.min(3) // Max 3 backends visible
        } else {
            10
        };

        let mut constraints = Vec::new();
        for _ in 0..backend_count.min(3) {
            constraints.push(Constraint::Length(height_per_backend as u16));
        }
        if backend_count > 3 {
            constraints.push(Constraint::Min(0));
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let backends: Vec<_> = snap.server_status.backends.iter().collect();
        for (i, (name, stats)) in backends.iter().enumerate().skip(scroll_offset).take(3) {
            if i - scroll_offset < chunks.len() {
                render_backend_detail(f, chunks[i - scroll_offset], name, stats);
            }
        }

        // Show scroll hint if needed
        if backend_count > 3 {
            let hint = Paragraph::new(format!(
                "Showing backends {}-{} of {} (use ↑↓ to scroll)",
                scroll_offset + 1,
                (scroll_offset + 3).min(backend_count),
                backend_count
            ))
            .style(Style::default().fg(Color::Gray));

            if let Some(last_chunk) = chunks.last() {
                f.render_widget(hint, *last_chunk);
            }
        }
    } else {
        render_loading(f, area);
    }
}

fn render_backend_detail(
    f: &mut Frame,
    area: Rect,
    name: &str,
    stats: &crate::metrics::collector::BackendStats,
) {
    let error_rate = if stats.requests > 0 {
        (stats.errors as f64 / stats.requests as f64) * 100.0
    } else {
        0.0
    };

    let status = if stats.errors == 0 {
        ("● Healthy", Color::Green)
    } else if error_rate > 5.0 {
        ("● Degraded", Color::Red)
    } else {
        ("⚠ Warning", Color::Yellow)
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(status.0, Style::default().fg(status.1).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Requests:         ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_number(stats.requests),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Errors:           ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} ({:.2}%)", stats.errors, error_rate),
                if stats.errors > 0 {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Avg Response:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}ms", stats.avg_response_ms),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Success Rate:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}%", 100.0 - error_rate),
                if error_rate < 1.0 {
                    Style::default().fg(Color::Green)
                } else if error_rate < 5.0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
    ];

    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Backend: {}", name))
            .border_style(Style::default().fg(status.1)),
    );

    f.render_widget(widget, area);
}

fn render_no_backends(f: &mut Frame, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No backends configured",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Check your configuration file to add backends.",
            Style::default().fg(Color::Gray),
        )),
    ];

    let widget = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Backends"));

    f.render_widget(widget, area);
}

fn render_loading(f: &mut Frame, area: Rect) {
    let widget = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Loading backend information...",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title("Backends"));

    f.render_widget(widget, area);
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
