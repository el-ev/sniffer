use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use pcap::{Capture, Device};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use tokio::sync::mpsc;

use crate::{
    action::Action,
    component::{Component, ComponentRender},
    pages::filter::FilterDialog,
    tui::Event,
    data::packet::{PacketInfo, parse_packet},
};

pub struct SnifferPage {
    device_name: Option<String>,
    packets: Vec<PacketInfo>,
    is_capturing: bool,
    capture_start_time: std::time::SystemTime,
    status_message: String,
    action_tx: Option<mpsc::UnboundedSender<Action>>,
    packet_count: usize,
    scroll_position: usize,
    following: bool,
    filter_dialog: FilterDialog,
    current_filter: Option<String>,
    packet_rx: Option<mpsc::UnboundedReceiver<PacketInfo>>,
    capture_thread_handle: Option<thread::JoinHandle<()>>,
    stop_capture_flag: Arc<AtomicBool>,
    selected_packet: Option<usize>, // New field for selected packet index
}

impl Default for SnifferPage {
    fn default() -> Self {
        Self {
            device_name: None,
            packets: Vec::new(),
            is_capturing: false,
            capture_start_time: std::time::SystemTime::now(),
            status_message: "No device selected. Press 'D' to select a device.".to_string(),
            action_tx: None,
            packet_count: 0,
            scroll_position: 0,
            following: false,
            filter_dialog: FilterDialog::new(),
            current_filter: None,
            packet_rx: None,
            capture_thread_handle: None,
            stop_capture_flag: Arc::new(AtomicBool::new(false)),
            selected_packet: None, // Initialize as None
        }
    }
}

impl SnifferPage {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn set_device(&mut self, device_name: String) {
        self.device_name = Some(device_name.clone());
        self.status_message = format!(
            "Device set to: {device_name}. Press 'S' to start capturing."
        );
    }

    fn start_capture(&mut self) -> Result<()> {
        if let Some(ref device_name) = self.device_name {
            self.status_message = "Starting packet capture...".to_string();

            let devices = Device::list().context("Failed to list devices")?;
            let device = devices
                .iter()
                .find(|d| d.name == *device_name)
                .context("Device not found")?;

            let mut cap = Capture::from_device(device.clone())?
                .promisc(true)
                .snaplen(5000)
                .timeout(1000)
                .open()?;

            if let Some(ref filter) = self.current_filter {
                if !filter.is_empty() {
                    match cap.filter(filter, true) {
                        Ok(_) => {
                            self.status_message = format!(
                                "Capturing packets on {device_name} with filter: {filter}. Press 'S' to stop."
                            );
                        }
                        Err(e) => {
                            self.status_message =
                                format!("Filter error: {e}. Capturing without filter.");
                        }
                    }
                } else {
                    self.status_message =
                        format!("Capturing packets on {device_name}. Press 'S' to stop.");
                }
            } else {
                self.status_message =
                    format!("Capturing packets on {device_name}. Press 'S' to stop.");
            }

            let (packet_tx, packet_rx) = mpsc::unbounded_channel();
            self.packet_rx = Some(packet_rx);

            self.stop_capture_flag.store(false, Ordering::Relaxed);
            let stop_flag = Arc::clone(&self.stop_capture_flag);
            let capture_start_time = std::time::SystemTime::now();

            let handle = thread::spawn(move || {
                let mut packet_id = 0;
                while !stop_flag.load(Ordering::Relaxed) {
                    if let Ok(packet) = cap.next_packet() {
                        packet_id += 1;

                        let timestamp = format!(
                            "{:.6}",
                            std::time::SystemTime::now()
                                .duration_since(capture_start_time)
                                .unwrap_or_default()
                                .as_secs_f64()
                        );

                        let packet_info = parse_packet(packet_id, timestamp, packet.data.into());

                        if packet_tx.send(packet_info).is_err() {
                            break;
                        }
                    }
                }
            });

            self.capture_thread_handle = Some(handle);
            self.is_capturing = true;
            self.capture_start_time = std::time::SystemTime::now();
            self.packets.clear();
            self.packet_count = 0;
            self.scroll_position = 0;
        }
        Ok(())
    }

    fn stop_capture(&mut self) {
        self.stop_capture_flag.store(true, Ordering::Relaxed);
        self.is_capturing = false;

        // Wait for capture thread to finish
        if let Some(handle) = self.capture_thread_handle.take() {
            let _ = handle.join();
        }

        self.packet_rx = None;

        if let Some(ref device_name) = self.device_name {
            self.status_message = format!(
                "Stopped capturing on {}. Captured {} packets.",
                device_name, self.packet_count
            );
        }
    }

    fn receive_packets(&mut self) {
        if let Some(ref mut packet_rx) = self.packet_rx {
            while let Ok(packet) = packet_rx.try_recv() {
                self.packet_count += 1;
                self.packets.push(packet);
            }
        }
    }

    fn render_packet_list(&self, f: &mut Frame, area: Rect) {
        let header = ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<6}", "No."),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<15}", "Timestamp"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:10}", "Protocol"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<10}", "Length"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<47}", "Source"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<47}", "Destination"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let mut items = vec![header];

        let visible_start = self.scroll_position;
        let visible_end = std::cmp::min(
            visible_start + (area.height as usize).saturating_sub(3),
            self.packets.len(),
        );

        let packet_items: Vec<ListItem> = self
            .packets
            .iter()
            .enumerate()
            .skip(visible_start)
            .take(visible_end - visible_start)
            .map(|(i, packet)| {
                let is_selected = self.selected_packet == Some(visible_start + i);
                let base_style = if is_selected {
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let source_str = if let Some(src_ip) = packet.src_ip {
                    if let Some(src_port) = packet.src_port {
                        if src_ip.is_ipv6() {
                            format!("[{src_ip}]:{src_port}")
                        } else {
                            format!("{src_ip}:{src_port}")
                        }
                    } else {
                        src_ip.to_string()
                    }
                } else {
                    "N/A".to_string()
                };
                let destination_str = if let Some(dst_ip) = packet.dst_ip {
                    if let Some(dst_port) = packet.dst_port {
                        if dst_ip.is_ipv6() {
                            format!("[{dst_ip}]:{dst_port}")
                        } else {
                            format!("{dst_ip}:{dst_port}")
                        }
                    } else {
                        dst_ip.to_string()
                    }
                } else {
                    "N/A".to_string()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<6}", packet.id),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Yellow
                        }),
                    ),
                    Span::styled(
                        format!("{:<15}", packet.timestamp.split('.').next().unwrap_or("")),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Gray
                        }),
                    ),
                    Span::styled(
                        format!("{:<10}", &packet.protocol[..8.min(packet.protocol.len())]),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Cyan
                        }),
                    ),
                    Span::styled(
                        format!("{:<10}", packet.length),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Green
                        }),
                    ),
                    Span::styled(
                        format!("{source_str:<47}"),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Magenta
                        }),
                    ),
                    Span::styled(
                        format!("{destination_str:<47}"),
                        base_style.fg(if is_selected {
                            Color::White
                        } else {
                            Color::Magenta
                        }),
                    ),
                ]);
                ListItem::new(line).style(base_style)
            })
            .collect();

        items.extend(packet_items);

        let list = List::new(items).block(
            Block::default()
                .title(format!("Captured Packets ({})", self.packet_count))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        f.render_widget(list, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_color = if self.is_capturing {
            Color::Green
        } else if self.device_name.is_some() {
            Color::Yellow
        } else {
            Color::Red
        };

        let status = Paragraph::new(self.status_message.clone())
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(status_color))
            .wrap(Wrap { trim: true });

        f.render_widget(status, area);
    }
    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.is_capturing && !self.following {
            "S: Stop Capture  C: Clear Packets  ↑/↓: Scroll  F: Follow    Home/End: Jump  A: Filter  D: Device Selection  Enter: Open Packet  Q/Esc: Home"
        } else if self.is_capturing && self.following {
            "S: Stop Capture  C: Clear Packets  ↑/↓: Scroll  F: Unfollow  Home/End: Jump  A: Filter  D: Device Selection  Enter: Open Packet  Q/Esc: Home"
        } else if self.device_name.is_some() {
            "S: Start Capture  C: Clear Packets  A: Filter  D: Device Selection  Enter: Open Packet  Q/Esc: Home"
        } else {
            "A: Filter  D: Device Selection  Enter: Open Packet  Q/Esc: Home"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(help, area);
    }
    fn handle_mouse_click(&mut self, x: u16, y: u16, area: Rect) {
        // Check if click is within the packet list area
        if x > area.x
            && x < area.x + area.width - 1
            && y > area.y + 1
            && y < area.y + area.height - 1
        {
            let clicked_row = (y - area.y - 2) as usize; // -2 for border and header
            let packet_index = self.scroll_position + clicked_row;

            if packet_index < self.packets.len() {
                if self.selected_packet == Some(packet_index) {
                    // Double-click behavior: open packet details
                    if let Some(tx) = &self.action_tx {
                        let _ = tx.send(Action::PacketSelected(packet_index));
                    }
                } else {
                    // Single-click behavior: select packet
                    self.selected_packet = Some(packet_index);
                }
            }
        }
    }

    fn select_packet(&mut self, index: usize) {
        if index < self.packets.len() {
            self.selected_packet = Some(index);

            // Ensure selected packet is visible
            let visible_start = self.scroll_position;
            let visible_end = visible_start + 20; // Approximate visible area

            if index < visible_start {
                self.scroll_position = index;
            } else if index >= visible_end {
                self.scroll_position = index.saturating_sub(19);
            }
        }
    }

    pub fn get_packet(&self, index: usize) -> Option<PacketInfo> {
        if index < self.packets.len() {
            Some(self.packets[index].clone())
        } else {
            None
        }
    }
}

impl Component for SnifferPage {
    fn register_action_handler(&mut self, tx: mpsc::UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx.clone());
        self.filter_dialog.register_action_handler(tx)?;
        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        if self.filter_dialog.is_open
            && let Some(action) = self.filter_dialog.handle_events(event.clone())? {
                return Ok(Some(action));
            }

        let r = match event {
            Event::Tick => {
                if self.is_capturing {
                    self.receive_packets();
                }
                None
            }
            Event::Key(key_event) => self.handle_key_events(key_event)?,
            Event::Mouse(mouse_event) => {
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let area = Rect {
                            x: 0,
                            y: 0,
                            width: 100, // Will be updated during render
                            height: 100,
                        };
                        self.handle_mouse_click(mouse_event.column, mouse_event.row, area);
                    }
                    MouseEventKind::ScrollUp => {
                        if self.scroll_position > 0 {
                            self.scroll_position = self.scroll_position.saturating_sub(3);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if self.scroll_position + 20 < self.packets.len() {
                            self.scroll_position += 3;
                        }
                    }
                    _ => {}
                }
                None
            }
        };
        Ok(r)
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Char('s') => {
                if self.device_name.is_some() {
                    if self.is_capturing {
                        self.stop_capture();
                    } else {
                        self.start_capture()?;
                    }
                } else {
                    self.status_message =
                        "No device selected. Press 'd' to select a device.".to_string();
                }
            }
            KeyCode::Char('q') => {
                if self.is_capturing {
                    self.stop_capture();
                }
                return Ok(Some(Action::NavigateToHome));
            }
            KeyCode::Char('d') => {
                return Ok(Some(Action::NavigateToDevice));
            }
            KeyCode::Char('a') => {
                if self.is_capturing {
                    self.stop_capture();
                }
                self.filter_dialog.open();
            }
            KeyCode::Char('c') => {
                self.packets.clear();
                self.packet_count = 0;
                self.scroll_position = 0;
                self.selected_packet = None; // Clear selection
                self.status_message = "Cleared packet list.".to_string();
            }
            KeyCode::Char('f') => {
                self.following = !self.following;
            }
            KeyCode::Enter => {
                if let Some(selected_index) = self.selected_packet {
                    return Ok(Some(Action::PacketSelected(selected_index)));
                }
            }
            KeyCode::Up => {
                if !self.packets.is_empty() {
                    if let Some(current) = self.selected_packet {
                        if current > 0 {
                            self.select_packet(current - 1);
                        }
                    } else {
                        self.select_packet(0);
                    }
                } else if self.scroll_position > 0 {
                    self.scroll_position -= 1;
                }
            }
            KeyCode::Down => {
                if !self.packets.is_empty() {
                    if let Some(current) = self.selected_packet {
                        if current < self.packets.len() - 1 {
                            self.select_packet(current + 1);
                        }
                    } else {
                        self.select_packet(0);
                    }
                } else if self.scroll_position + 20 < self.packets.len() {
                    self.scroll_position += 1;
                }
            }
            KeyCode::Home => {
                if !self.packets.is_empty() {
                    self.select_packet(0);
                } else {
                    self.scroll_position = 0;
                }
            }
            KeyCode::End => {
                if !self.packets.is_empty() {
                    self.select_packet(self.packets.len() - 1);
                } else if self.packets.len() > 20 {
                    self.scroll_position = self.packets.len() - 20;
                } else {
                    self.scroll_position = 0;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::DeviceSelected(device_name) => {
                self.set_device(device_name);
            }
            Action::ApplyFilter(filter) => {
                self.current_filter = if filter.is_empty() {
                    None
                } else {
                    Some(filter.clone())
                };

                if let Some(ref filter_text) = self.current_filter {
                    self.status_message = format!("Filter applied: {filter_text}");
                } else {
                    self.status_message = "Filter cleared".to_string();
                }

                self.status_message
                    .push_str(". Press 'S' to start capturing.");
            }
            Action::PacketSelected(index) => {
                if index < self.packets.len() {
                    self.status_message = format!(
                        "Opening packet details for packet #{}",
                        self.packets[index].id
                    );
                    
                }
            }
            _ => {}
        }
        Ok(None)
    }
}

impl ComponentRender<()> for SnifferPage {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        if self.following && self.is_capturing {
            self.scroll_position = self
                .packets
                .len()
                .saturating_sub(chunks[0].height as usize - 3);
        }

        // Update the mouse click area with actual render area
        if let Some((x, y)) = std::mem::take(&mut None) {
            // This would be set by mouse events
            self.handle_mouse_click(x, y, chunks[0]);
        }

        self.render_packet_list(f, chunks[0]);
        self.render_status(f, chunks[1]);
        self.render_help(f, chunks[2]);
        if self.filter_dialog.is_open {
            self.filter_dialog.render(f, area, ());
        }
    }
}
