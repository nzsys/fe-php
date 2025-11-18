use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, _scroll_offset: usize) {
    let content = vec![
        Line::from(vec![
            Span::styled(
                "fe-php TUI Monitor - Keyboard Shortcuts",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Tab / →       ", Style::default().fg(Color::Green)),
            Span::raw("Next tab"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab / ← ", Style::default().fg(Color::Green)),
            Span::raw("Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / ↓         ", Style::default().fg(Color::Green)),
            Span::raw("Scroll up/down (in scrollable tabs)"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc       ", Style::default().fg(Color::Green)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  r             ", Style::default().fg(Color::Green)),
            Span::raw("Refresh data manually"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+R       ", Style::default().fg(Color::Magenta)),
            Span::raw("Reload configuration (requires --socket)"),
        ]),
        Line::from(vec![
            Span::styled("  W             ", Style::default().fg(Color::Magenta)),
            Span::raw("Restart workers (requires --socket)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tabs", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Overview      ", Style::default().fg(Color::Cyan)),
            Span::raw("Server status and summary"),
        ]),
        Line::from(vec![
            Span::styled("  Metrics       ", Style::default().fg(Color::Cyan)),
            Span::raw("Request metrics and backend statistics"),
        ]),
        Line::from(vec![
            Span::styled("  Backends      ", Style::default().fg(Color::Cyan)),
            Span::raw("Detailed backend information"),
        ]),
        Line::from(vec![
            Span::styled("  Security      ", Style::default().fg(Color::Cyan)),
            Span::raw("WAF, IP blocker, GeoIP status"),
        ]),
        Line::from(vec![
            Span::styled("  Logs          ", Style::default().fg(Color::Cyan)),
            Span::raw("Recent log entries (scrollable)"),
        ]),
        Line::from(vec![
            Span::styled("  Analysis      ", Style::default().fg(Color::Cyan)),
            Span::raw("Log analysis and statistics"),
        ]),
        Line::from(vec![
            Span::styled("  Help          ", Style::default().fg(Color::Cyan)),
            Span::raw("This help screen"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Connection Modes", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Local         ", Style::default().fg(Color::Green)),
            Span::raw("fe-php monitor (same machine)"),
        ]),
        Line::from(vec![
            Span::styled("  Remote        ", Style::default().fg(Color::Green)),
            Span::raw("fe-php monitor --socket /path/to/socket"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Color::Yellow)),
            Span::raw("Interactive operations (Shift+R, W) require remote connection via --socket"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::Gray)),
            Span::raw("fe-php v0.1.0"),
        ]),
    ];

    let widget = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(widget, area);
}
