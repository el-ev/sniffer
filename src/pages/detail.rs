use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use tokio::sync::mpsc;

use crate::{
    action::Action,
    component::{Component, ComponentRender},
    data::packet::PacketInfo,
    tui::Event,
};

#[derive(Default)]
pub struct PacketDetailsPage {
    packet: Option<PacketInfo>,
    hex_scroll: usize,
    action_tx: Option<mpsc::UnboundedSender<Action>>,
}

impl PacketDetailsPage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_packet(&mut self, packet: PacketInfo) {
        self.packet = Some(packet);
        self.hex_scroll = 0;
    }

    fn render_packet_info(&self, f: &mut Frame, area: Rect) {
        if let Some(ref packet) = self.packet {
            let info_lines = vec![
                Line::from(vec![
                    Span::styled(
                        "Packet ID: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(packet.id.to_string(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Timestamp: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(packet.timestamp.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Protocol: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(packet.protocol.clone(), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Length: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} bytes", packet.length),
                        Style::default().fg(Color::Green),
                    ),
                ]),
            ];

            let mut info_text = info_lines;

            if let Some(ref src) = packet.src_addr {
                match src {
                    Ok(src_ip) => {
                        let src_line = if let Some(src_port) = packet.src_port {
                            Line::from(vec![
                                Span::styled(
                                    "Source: ",
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!("{src_ip}:{src_port}"),
                                    Style::default().fg(Color::Magenta),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::styled(
                                    "Source IP: ",
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(src_ip.to_string(), Style::default().fg(Color::Magenta)),
                            ])
                        };
                        info_text.push(src_line);
                    }
                    Err(src_mac) => {
                        let src_line = Line::from(vec![
                            Span::styled(
                                "Source MAC: ",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(src_mac, Style::default().fg(Color::Magenta)),
                        ]);
                        info_text.push(src_line);
                    }
                }
            }

            if let Some(ref dst) = packet.dst_addr {
                match dst {
                    Ok(dst_ip) => {
                        let dst_line = if let Some(dst_port) = packet.dst_port {
                            Line::from(vec![
                                Span::styled(
                                    "Destination: ",
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!("{dst_ip}:{dst_port}"),
                                    Style::default().fg(Color::Magenta),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::styled(
                                    "Destination IP: ",
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(dst_ip.to_string(), Style::default().fg(Color::Magenta)),
                            ])
                        };
                        info_text.push(dst_line);
                    }
                    Err(dst_mac) => {
                        let dst_line = Line::from(vec![
                            Span::styled(
                                "Destination MAC: ",
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(dst_mac, Style::default().fg(Color::Magenta)),
                        ]);
                        info_text.push(dst_line);
                    }
                }
            }

            let paragraph = Paragraph::new(info_text)
                .block(
                    Block::default()
                        .title(" Packet Information")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, area);
        } else {
            let no_packet = Paragraph::new("No packet selected")
                .block(
                    Block::default()
                        .title(" Packet Information")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                )
                .style(Style::default().fg(Color::Gray));

            f.render_widget(no_packet, area);
        }
    }

    fn render_hex_viewer(&self, f: &mut Frame, area: Rect) {
        if self.packet.is_none() {
            let no_packet = Paragraph::new("No packet selected")
                .block(
                    Block::default()
                        .title(" Hex Viewer")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                )
                .style(Style::default().fg(Color::Gray));

            f.render_widget(no_packet, area);
            return;
        }
        let packet = self.packet.as_ref().unwrap();
        let mut hex_lines = Vec::new();

        // Header
        hex_lines.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {:^9}", "Offset"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:^48}", "Hex"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:^16}", "ASCII"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ])));

        let bytes_per_line = 16;
        let visible_lines = (area.height as usize).saturating_sub(3); // Account for borders and header
        let start_offset = self.hex_scroll * bytes_per_line;
        let end_offset = std::cmp::min(
            start_offset + (visible_lines * bytes_per_line),
            packet.data.len(),
        );

        for offset in (start_offset..end_offset).step_by(bytes_per_line) {
            let end = std::cmp::min(offset + bytes_per_line, packet.data.len());
            let line_data = &packet.data[offset..end];

            let mut hex_str = String::new();
            let mut ascii_str = String::new();

            hex_str.push_str("      ");
            for (i, &byte) in line_data.iter().enumerate() {
                if i > 0 && i % 4 == 0 {
                    hex_str.push(' ');
                }
                hex_str.push_str(&format!("{byte:02x}"));

                // ASCII representation
                if byte.is_ascii_graphic() || byte == b' ' {
                    ascii_str.push(byte as char);
                } else {
                    ascii_str.push('.');
                }
            }

            // Pad hex string to maintain alignment
            while hex_str.len() < 47 {
                hex_str.push(' ');
            }

            let line = Line::from(vec![
                Span::styled(format!(" {offset:08x}"), Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(hex_str, Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::styled(ascii_str, Style::default().fg(Color::Cyan)),
            ]);

            hex_lines.push(ListItem::new(line));
        }

        let hex_list = List::new(hex_lines).block(
            Block::default()
                .title(format!(" Hex Viewer ({} bytes)", packet.data.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        f.render_widget(hex_list, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = "↑/↓: Scroll Hex  Q: Back to Sniffer  Esc: Back to Home";

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(help, area);
    }
}

impl Component for PacketDetailsPage {
    fn register_action_handler(&mut self, tx: mpsc::UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        match event {
            Event::Key(key_event) => self.handle_key_events(key_event),
            _ => Ok(None),
        }
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let packet = match self.packet {
            Some(ref p) => p,
            None => return Ok(None),
        };
        match key.code {
            KeyCode::Char('q') => {
                return Ok(Some(Action::NavigateToSniffer));
            }
            KeyCode::Up => {
                if self.hex_scroll > 0 {
                    self.hex_scroll -= 1;
                }
            }
            KeyCode::Down => {
                let max_scroll = (packet.data.len() / 16).saturating_sub(10);
                if self.hex_scroll < max_scroll {
                    self.hex_scroll += 1;
                }
            }
            KeyCode::PageUp => {
                self.hex_scroll = self.hex_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let max_scroll = (packet.data.len() / 16).saturating_sub(10);
                self.hex_scroll = std::cmp::min(self.hex_scroll + 10, max_scroll);
            }
            KeyCode::Home => {
                self.hex_scroll = 0;
            }
            KeyCode::End => {
                let max_scroll = (packet.data.len() / 16).saturating_sub(10);
                self.hex_scroll = max_scroll;
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
}

impl ComponentRender<()> for PacketDetailsPage {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Packet info
                Constraint::Min(10),   // Hex viewer
                Constraint::Length(1), // Help
            ])
            .split(area);

        self.render_packet_info(f, chunks[0]);
        self.render_hex_viewer(f, chunks[1]);
        self.render_help(f, chunks[2]);
    }
}
