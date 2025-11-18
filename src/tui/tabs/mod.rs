pub mod overview;
pub mod metrics;
pub mod backends;
pub mod security;
pub mod logs;
pub mod analysis;
pub mod help;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};

use crate::tui::app::ConnectionStatus;

pub fn render_tab_bar(
    f: &mut Frame,
    area: Rect,
    selected: usize,
    connection_status: &ConnectionStatus,
) {
    let titles = vec!["Overview", "Metrics", "Backends", "Security", "Logs", "Analysis", "Help"];

    // Create title with connection status
    let title = match connection_status {
        ConnectionStatus::Connected => {
            Line::from(vec![
                Span::raw("fe-php Monitor "),
                Span::styled("● Connected", Style::default().fg(Color::Green)),
            ])
        }
        ConnectionStatus::Connecting => {
            Line::from(vec![
                Span::raw("fe-php Monitor "),
                Span::styled("● Connecting...", Style::default().fg(Color::Yellow)),
            ])
        }
        ConnectionStatus::Disconnected(_) => {
            Line::from(vec![
                Span::raw("fe-php Monitor "),
                Span::styled("● Disconnected", Style::default().fg(Color::Red)),
            ])
        }
    };

    let tabs = Tabs::new(
        titles
            .iter()
            .map(|t| Line::from(vec![Span::raw(*t)]))
            .collect::<Vec<_>>(),
    )
    .block(Block::default().borders(Borders::ALL).title(title))
    .select(selected)
    .style(Style::default().fg(Color::White))
    .highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(tabs, area);
}
