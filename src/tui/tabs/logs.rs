use crate::monitor::LogAnalyzer;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(
    f: &mut Frame,
    area: Rect,
    analyzer: &LogAnalyzer,
    scroll_offset: usize,
) {
    let recent_logs = analyzer.get_recent_logs(100);

    let items: Vec<ListItem> = recent_logs
        .iter()
        .rev() // Show newest first
        .skip(scroll_offset)
        .take(area.height as usize - 2) // Account for borders
        .map(|log| {
            let status_color = match log.status {
                200..=299 => Color::Green,
                300..=399 => Color::Cyan,
                400..=499 => Color::Yellow,
                500..=599 => Color::Red,
                _ => Color::White,
            };

            let content = Line::from(vec![
                Span::styled(
                    log.timestamp.format("%H:%M:%S").to_string(),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:4}", log.status),
                    Style::default().fg(status_color),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:6}", log.method),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::styled(&log.uri, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(
                    format!("{}ms", log.duration_ms),
                    if log.duration_ms > 100 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let title = format!(
        "Recent Logs (showing {}/{}, scroll: {}) - [↑/↓] to scroll",
        items.len(),
        recent_logs.len(),
        scroll_offset
    );

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}
