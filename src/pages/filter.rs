use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use tokio::sync::mpsc;

use crate::{
    action::Action,
    component::{Component, ComponentRender},
    tui::Event,
};

pub struct FilterDialog {
    pub is_open: bool,
    pub filter_text: String,
    pub cursor_position: usize,
    pub selected_preset: usize,
    pub mode: FilterMode,
    action_tx: Option<mpsc::UnboundedSender<Action>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterMode {
    CustomInput,
    PresetSelection,
}

impl Default for FilterDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            filter_text: String::new(),
            cursor_position: 0,
            selected_preset: 0,
            mode: FilterMode::CustomInput,
            action_tx: None,
        }
    }
}

impl FilterDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.filter_text.clear();
        self.cursor_position = 0;
        self.selected_preset = 0;
        self.mode = FilterMode::CustomInput;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn get_filter_presets() -> Vec<(&'static str, &'static str)> {
        vec![
            ("TCP Traffic", "tcp"),
            ("UDP Traffic", "udp"),
            ("HTTP Traffic", "tcp port 80 or tcp port 8080"),
            ("HTTPS Traffic", "tcp port 443"),
            ("DNS Traffic", "udp port 53 or tcp port 53"),
            ("SSH Traffic", "tcp port 22"),
            ("FTP Traffic", "tcp port 21 or tcp port 20"),
            ("ICMP Traffic", "icmp"),
            ("ARP Traffic", "arp"),
            ("IPv6 Traffic", "ip6"),
            ("Broadcast", "broadcast"),
            ("Multicast", "multicast"),
            ("Large Packets", "greater 1000"),
            ("Small Packets", "less 100"),
            ("Clear Filter", ""),
        ]
    }

    fn apply_filter(&mut self, filter: String) {
        if let Some(ref tx) = self.action_tx {
            let _ = tx.send(Action::ApplyFilter(filter));
        }
        self.close();
    }

    fn render_custom_input(&self, f: &mut Frame, area: Rect) {
        let input_block = Block::default()
            .title("Enter Custom Filter")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let input_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Input field
        let input = Paragraph::new(self.filter_text.as_str())
            .block(input_block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        f.render_widget(input, input_area[0]);

        // Help text
        let help_text = vec![
            Line::from("Examples:"),
            Line::from("  tcp port 80        - HTTP traffic"),
            Line::from("  udp port 53        - DNS traffic"),
            Line::from("  host 192.168.1.1   - Traffic to/from specific host"),
            Line::from("  net 192.168.1.0/24 - Traffic from subnet"),
            Line::from("  icmp               - ICMP packets"),
            Line::from(""),
            Line::from("Tab: Switch to presets  Enter: Apply  Esc: Cancel"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().title("Help").borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: false });

        f.render_widget(help, input_area[1]);        // Show cursor
        if self.mode == FilterMode::CustomInput {
            let cursor_x = input_area[0].x + 1 + self.cursor_position as u16;
            let cursor_y = input_area[0].y + 1;
            if cursor_x < input_area[0].x + input_area[0].width - 1 {
                f.set_cursor_position(ratatui::layout::Position { x: cursor_x, y: cursor_y });
            }
        }
    }

    fn render_preset_selection(&self, f: &mut Frame, area: Rect) {
        let presets = Self::get_filter_presets();
        
        let items: Vec<ListItem> = presets
            .iter()
            .enumerate()
            .map(|(i, (name, filter))| {
                let style = if i == self.selected_preset {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };

                let line = if filter.is_empty() {
                    Line::from(vec![
                        Span::styled(format!("{name:<20}"), style),
                        Span::styled("(removes current filter)", Style::default().fg(Color::Gray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(format!("{name:<20}"), style),
                        Span::styled(format!("- {filter}"), Style::default().fg(Color::Gray)),
                    ])
                };
                
                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Filter Presets")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .highlight_style(Style::default().bg(Color::Blue));

        f.render_widget(list, area);

        // Help text at bottom
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 3,
            width: area.width,
            height: 3,
        };

        let help = Paragraph::new("Tab: Switch to custom input  Enter: Apply  ↑/↓: Navigate  Esc: Cancel")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(help, help_area);
    }
}

impl Component for FilterDialog {
    fn register_action_handler(&mut self, tx: mpsc::UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Event) -> Result<Option<Action>> {
        if let Event::Key(key) = event {
            self.handle_key_events(key)
        } else {
            Ok(None)
        }
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.close();
                Ok(Some(Action::Handled))
            }
            KeyCode::Tab => {
                self.mode = match self.mode {
                    FilterMode::CustomInput => FilterMode::PresetSelection,
                    FilterMode::PresetSelection => FilterMode::CustomInput,
                };
                Ok(Some(Action::Handled))
            }
            KeyCode::Enter => {
                match self.mode {
                    FilterMode::CustomInput => {
                        let filter = self.filter_text.clone();
                        self.apply_filter(filter);
                    }
                    FilterMode::PresetSelection => {
                        let presets = Self::get_filter_presets();
                        if let Some((_, filter)) = presets.get(self.selected_preset) {
                            self.apply_filter(filter.to_string());
                        }
                    }
                }
                Ok(Some(Action::Handled))
            }
            _ => {
                match self.mode {
                    FilterMode::CustomInput => self.handle_custom_input(key),
                    FilterMode::PresetSelection => self.handle_preset_sel(key),
                }
            }
        }
    }

    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
}

impl FilterDialog {
    fn handle_custom_input(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Char(c) => {
                self.filter_text.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 && !self.filter_text.is_empty() {
                    self.cursor_position -= 1;
                    self.filter_text.remove(self.cursor_position);
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.filter_text.len() {
                    self.filter_text.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.filter_text.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.filter_text.len();
            }
            _ => {}
        }
        Ok(Some(Action::Handled))
    }

    fn handle_preset_sel(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let presets = Self::get_filter_presets();
        match key.code {
            KeyCode::Up => {
                if self.selected_preset > 0 {
                    self.selected_preset -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_preset < presets.len() - 1 {
                    self.selected_preset += 1;
                }
            }
            KeyCode::Home => {
                self.selected_preset = 0;
            }
            KeyCode::End => {
                self.selected_preset = presets.len().saturating_sub(1);
            }
            _ => {}
        }
        Ok(Some(Action::Handled))
    }
}

impl ComponentRender<()> for FilterDialog {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        if !self.is_open {
            return;
        }

        // Create centered popup
        let popup_area = {
            let percent_x = 80;
            let percent_y = 70;
            let popup_width = area.width * percent_x / 100;
            let popup_height = area.height * percent_y / 100;
            let popup_x = (area.width - popup_width) / 2;
            let popup_y = (area.height - popup_height) / 2;

            Rect {
                x: popup_x,
                y: popup_y,
                width: popup_width,
                height: popup_height,
            }
        };

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Render background block
        let bg_block = Block::default()
            .title("Packet Filter")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(Color::Black));

        f.render_widget(bg_block, popup_area);

        // Inner area for content
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        match self.mode {
            FilterMode::CustomInput => self.render_custom_input(f, inner_area),
            FilterMode::PresetSelection => self.render_preset_selection(f, inner_area),
        }
    }
}