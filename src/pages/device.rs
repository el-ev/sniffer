use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use pcap::Device;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    action::Action,
    component::{Component, ComponentRender},
    tui::Event,
};

#[derive(Default)]
pub struct DevicePage {
    devices: Vec<Device>,
    list_state: ListState,
    selected_device: Option<Device>,
    status_message: String,
    loading: bool,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    mouse_event: Option<(u16, u16)>,
}

impl DevicePage {
    pub fn new() -> Self {
        Self::default()
    }

    fn load_devices(&mut self) -> Result<()> {
        self.loading = true;
        self.status_message = "Probing network devices...".to_string();

        match Device::list() {
            Ok(devices) => {
                if devices.is_empty() {
                    self.status_message = "No network devices found.".to_string();
                } else {
                    self.status_message = format!(
                        "Found {} device(s). Use ↑/↓ to navigate, Enter to select.",
                        devices.len()
                    );
                    self.devices = devices;
                    if !self.devices.is_empty() {
                        self.list_state.select(Some(1)); // 0 is the header
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Failed to list devices: {e}");
            }
        }

        self.loading = false;
        Ok(())
    }

    fn select_current_device(&mut self) {
        if let Some(selected) = self.list_state.selected()
            && selected <= self.devices.len() {
                self.selected_device = Some(self.devices[selected - 1].clone());
                self.status_message = format!("Selected device: {}", self.devices[selected].name);
                if let Some(tx) = &self.action_tx {
                    let action = Action::DeviceSelected(self.devices[selected - 1].name.clone());
                    if tx.send(action).is_err() {
                        self.status_message = "Failed to send device selection action.".to_string();
                    }
                }
            }
    }

    fn clear_selection(&mut self) {
        self.list_state.select(None);
        self.selected_device = None;
        if !self.devices.is_empty() {
            self.status_message = format!("Found {} device(s)", self.devices.len());
        }
    }

    fn render_device_list(&self, f: &mut Frame, area: Rect) {
        if self.devices.is_empty() {
            let empty_message = Paragraph::new("No devices found. Press F5 to refresh.")
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Network Devices"),
                )
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });

            f.render_widget(empty_message, area);
            return;
        }

        let header = ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<4}", "No."),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<80}", "Description"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Name",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let mut items = vec![header];

        items.extend(self.devices.iter().enumerate().map(|(i, device)| {
            const DEFAULT_DESC: &str = "No description";
            let truncated_desc = if let Some(desc) = device.desc.as_deref() {
                if desc.len() > 76 { &desc[..76] } else { desc }
            } else {
                DEFAULT_DESC
            };

            let line = Line::from(vec![
                Span::styled(format!("{:<4}", i + 1), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{truncated_desc:<80}"),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(&device.name, Style::default().fg(Color::Cyan)),
            ]);
            ListItem::new(line)
        }));

        let selected_style = if self.selected_device.is_some() {
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Network Devices")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(selected_style);

        f.render_stateful_widget(list, area, &mut self.list_state.clone());
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_color = if self.loading {
            Color::Yellow
        } else if self.devices.is_empty() && !self.status_message.contains("Found") {
            Color::Red
        } else {
            Color::Green
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
        let help_text = if self.selected_device.is_some() {
            "↑/↓: Navigate  Enter: Select Device  Q/Esc: Home  B: Back  F5: Refresh  C: Clear Selection"
        } else {
            "↑/↓: Navigate  Enter: Select Device  Q/Esc: Home  B: Back  F5: Refresh"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(help, area);
    }

    fn handle_mouse_click(&mut self, x: u16, y: u16, area: Rect) {
        if x >= area.x && x < area.x + area.width && y > area.y + 1 && y < area.y + area.height - 1
        {
            let clicked_index = (y - area.y - 2) as usize; // -2 border and header
            if clicked_index < self.devices.len() {
                if self.list_state.selected() == Some(clicked_index + 1) {
                    self.select_current_device();
                } else {
                    self.list_state.select(Some(clicked_index + 1));
                }
            }
        }
    }
}

impl Component for DevicePage {
    fn register_action_handler(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.load_devices()?;
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        let r = match event {
            Event::Key(key_event) => self.handle_key_events(key_event)?,
            Event::Mouse(mouse_event) => {
                if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                    self.mouse_event = Some((mouse_event.column, mouse_event.row));
                }
                None
            }
            _ => None,
        };
        Ok(r)
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Up => {
                if !self.devices.is_empty() {
                    let current = self.list_state.selected().unwrap_or(1);
                    let i = if current <= 1 {
                        self.devices.len()
                    } else {
                        current - 1
                    };
                    self.list_state.select(Some(i));
                }
            }
            KeyCode::Down => {
                if !self.devices.is_empty() {
                    let current = self.list_state.selected().unwrap_or(0);
                    let i = if current >= self.devices.len() {
                        1
                    } else {
                        current + 1
                    };
                    self.list_state.select(Some(i));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected()
                    && selected > 0 && selected <= self.devices.len() {
                        self.select_current_device();
                    }
            }
            KeyCode::Char('c') => {
                self.clear_selection();
            }
            KeyCode::Char('b') => {
                return Ok(Some(Action::NavigateToHome));
            }
            KeyCode::Char('q') => {
                return Ok(Some(Action::NavigateToHome));
            }
            KeyCode::F(5) => {
                self.load_devices()?;
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
}

impl ComponentRender<()> for DevicePage {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        if let Some((x, y)) = self.mouse_event.take() {
            self.handle_mouse_click(x, y, chunks[0]);
        }

        self.render_device_list(f, chunks[0]);
        self.render_status(f, chunks[1]);
        self.render_help(f, chunks[2]);
    }
}
