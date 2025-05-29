use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
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
pub struct HomePage {
    list_state: ListState,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    mouse_event: Option<(u16, u16)>,
}

impl HomePage {
    pub fn new() -> Self {
        let mut home = Self::default();
        home.list_state.select(Some(1));
        home
    }

    fn render_menu(&self, f: &mut Frame, area: Rect) {
        let header = ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<4}", "No."),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<20}", "Module"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Description",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let mut items = vec![header];

        let menu_items = vec![
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<4}", "1"), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:<20}", "Device Selection"),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "Select network interface for packet capture",
                    Style::default().fg(Color::Gray),
                ),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<4}", "2"), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:<20}", "Packet Sniffer"),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "Capture and analyze network packets",
                    Style::default().fg(Color::Gray),
                ),
            ])),
        ];

        items.extend(menu_items);

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Network Packet Sniffer")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.list_state.clone());
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status =
            Paragraph::new("Welcome to Network Packet Sniffer. Select a module to continue.")
                .block(
                    Block::default()
                        .title("Status")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Blue)),
                )
                .style(Style::default().fg(Color::Green))
                .wrap(Wrap { trim: true });

        f.render_widget(status, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = "↑/↓: Navigate  Enter: Select Module  D: Device Selection  S: Packet Sniffer  Q/Esc: Exit";

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true })
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(help, area);
    }

    fn handle_mouse_click(&mut self, x: u16, y: u16, area: Rect) -> Option<Action> {
        if x >= area.x && x < area.x + area.width && y > area.y + 1 && y < area.y + area.height - 1
        {
            let clicked_index = (y - area.y - 2) as usize;
            if clicked_index < 2 {
                let menu_item = clicked_index + 1;
                if self.list_state.selected() == Some(menu_item) {
                    match menu_item {
                        1 => return Some(Action::NavigateToDevice),
                        2 => return Some(Action::NavigateToSniffer),
                        _ => {}
                    }
                } else {
                    self.list_state.select(Some(menu_item));
                }
            }
        }
        None
    }
}

impl Component for HomePage {
    fn register_action_handler(
        &mut self,
        _tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
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
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i <= 1 {
                            2
                        } else {
                            i - 1
                        }
                    }
                    None => 1,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Down => {
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= 2 {
                            1
                        } else {
                            i + 1
                        }
                    }
                    None => 1,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Enter => match self.list_state.selected() {
                Some(1) => return Ok(Some(Action::NavigateToDevice)),
                Some(2) => return Ok(Some(Action::NavigateToSniffer)),
                _ => {}
            },
            KeyCode::Char('d') => return Ok(Some(Action::NavigateToDevice)),
            KeyCode::Char('s') => return Ok(Some(Action::NavigateToSniffer)),
            KeyCode::Char('q') => {
                return Ok(Some(Action::Quit));
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> Result<Option<Action>> {
        Ok(None)
    }
}

impl ComponentRender<()> for HomePage {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        if let Some((x, y)) = self.mouse_event.take() {
            let action = self.handle_mouse_click(x, y, chunks[0]);
            if let Some(action) = action {
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(action);
                }
            }
        }

        self.render_menu(f, chunks[0]);
        self.render_status(f, chunks[1]);
        self.render_help(f, chunks[2]);
    }
}
