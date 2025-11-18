use crate::monitor::{MonitorCollector, LogAnalyzer};
use crate::monitor::analyzer::LogAnalysisResult;
use crate::monitor::collector::MonitorSnapshot;
use crate::tui::client::TuiClient;
use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use std::sync::Arc;

pub enum Tab {
    Metrics,
    Logs,
    Analysis,
}

pub struct App {
    pub current_tab: usize,
    pub monitor: MonitorCollector,
    pub analyzer: LogAnalyzer,
    pub snapshot: Option<MonitorSnapshot>,
    pub analysis: Option<LogAnalysisResult>,
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub connection_status: ConnectionStatus,
    pub client: Option<Arc<TuiClient>>,  // For interactive operations
    pub status_message: Option<String>,  // For showing operation results
    pub blocked_ips: Vec<String>,  // List of blocked IPs
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected(String), // Error message
}

impl App {
    pub fn new(monitor: MonitorCollector) -> Self {
        Self {
            current_tab: 0,
            monitor,
            analyzer: LogAnalyzer::new(),
            snapshot: None,
            analysis: None,
            scroll_offset: 0,
            error_message: None,
            connection_status: ConnectionStatus::Connecting,
            client: None,
            status_message: None,
            blocked_ips: Vec::new(),
        }
    }

    /// Create app with client for interactive operations
    pub fn with_client(monitor: MonitorCollector, client: Arc<TuiClient>) -> Self {
        Self {
            current_tab: 0,
            monitor,
            analyzer: LogAnalyzer::new(),
            snapshot: None,
            analysis: None,
            scroll_offset: 0,
            error_message: None,
            connection_status: ConnectionStatus::Connecting,
            client: Some(client),
            status_message: None,
            blocked_ips: Vec::new(),
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % 7;
        self.scroll_offset = 0;
    }

    pub fn previous_tab(&mut self) {
        if self.current_tab > 0 {
            self.current_tab -= 1;
        } else {
            self.current_tab = 6;
        }
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub async fn refresh(&mut self) -> Result<()> {
        // Clear previous error and status messages
        self.error_message = None;
        self.status_message = None;

        // Try to take a snapshot
        match self.monitor.take_snapshot().await {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.connection_status = ConnectionStatus::Connected;
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                self.error_message = Some(error_msg.clone());
                self.connection_status = ConnectionStatus::Disconnected(error_msg);

                // Don't fail the refresh, just show the error
                // This allows the TUI to continue running and retry
                return Ok(());
            }
        }

        // Update analyzer with recent logs from snapshot
        if let Some(ref snapshot) = self.snapshot {
            // Clear and re-add logs from snapshot
            self.analyzer = LogAnalyzer::new();
            for log in &snapshot.server_status.recent_logs {
                self.analyzer.add_log(log.clone());
            }
        }

        // Analyze logs
        self.analysis = Some(self.analyzer.analyze());

        // Fetch blocked IPs if client is available
        if let Some(ref client) = self.client {
            if let Ok(blocked_ips) = client.get_blocked_ips().await {
                self.blocked_ips = blocked_ips;
            }
        }

        Ok(())
    }

    /// Reload configuration (interactive operation)
    pub async fn reload_config(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            match client.reload_config(None).await {
                Ok(msg) => {
                    self.status_message = Some(format!("✓ {}", msg));
                }
                Err(e) => {
                    self.status_message = Some(format!("✗ {}", e));
                }
            }
        } else {
            self.status_message = Some("✗ Interactive operations not available (not connected to server)".to_string());
        }
        Ok(())
    }

    /// Block IP address (interactive operation)
    pub async fn block_ip(&mut self, ip: String) -> Result<()> {
        if let Some(ref client) = self.client {
            match client.block_ip(ip).await {
                Ok(msg) => {
                    self.status_message = Some(format!("✓ {}", msg));
                }
                Err(e) => {
                    self.status_message = Some(format!("✗ {}", e));
                }
            }
        } else {
            self.status_message = Some("✗ Interactive operations not available (not connected to server)".to_string());
        }
        Ok(())
    }

    /// Restart workers (interactive operation)
    pub async fn restart_workers(&mut self) -> Result<()> {
        if let Some(ref client) = self.client {
            match client.restart_workers().await {
                Ok(msg) => {
                    self.status_message = Some(format!("✓ {}", msg));
                }
                Err(e) => {
                    self.status_message = Some(format!("✗ {}", e));
                }
            }
        } else {
            self.status_message = Some("✗ Interactive operations not available (not connected to server)".to_string());
        }
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame) {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),      // Tab bar
                    Constraint::Min(0),          // Content
                    Constraint::Length(if self.status_message.is_some() { 3 } else { 0 }), // Status bar
                ]
                .as_ref(),
            )
            .split(f.size());

        // Render tab bar with connection status
        super::tabs::render_tab_bar(f, chunks[0], self.current_tab, &self.connection_status);

        // If there's an error message, show it instead of the normal content
        if let Some(ref error_msg) = self.error_message {
            use ratatui::widgets::Paragraph;
            use ratatui::style::{Color, Style};
            use ratatui::text::{Line, Span};

            let error_widget = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Connection Error:",
                    Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(error_msg.as_str()),
                Line::from(""),
                Line::from(Span::styled(
                    "Retrying...",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from("Press 'r' to retry manually, 'q' to quit"),
            ])
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("Error"),
            );

            f.render_widget(error_widget, chunks[1]);
        } else {
            // Render selected tab content
            match self.current_tab {
                0 => super::tabs::overview::render(f, chunks[1], &self.snapshot, self.scroll_offset),
                1 => super::tabs::metrics::render(f, chunks[1], &self.snapshot, self.scroll_offset),
                2 => super::tabs::backends::render(f, chunks[1], &self.snapshot, self.scroll_offset),
                3 => super::tabs::security::render(f, chunks[1], &self.snapshot, &self.client, &self.blocked_ips, self.scroll_offset),
                4 => super::tabs::logs::render(f, chunks[1], &self.analyzer, self.scroll_offset),
                5 => super::tabs::analysis::render(f, chunks[1], &self.analysis, self.scroll_offset),
                6 => super::tabs::help::render(f, chunks[1], self.scroll_offset),
                _ => {}
            }
        }

        // Render status message (if any)
        if let Some(ref status_msg) = self.status_message {
            let status_color = if status_msg.starts_with("✓") {
                Color::Green
            } else {
                Color::Red
            };

            let status_widget = Paragraph::new(status_msg.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(status_color))
                        .title("Status"),
                )
                .style(Style::default().fg(status_color));

            f.render_widget(status_widget, chunks[2]);
        }
    }
}
