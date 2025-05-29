use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc;

use crate::{
    action::Action,
    component::{Component, ComponentRender},
    pages::{detail::PacketDetailsPage, device::DevicePage, home::HomePage, sniffer::SnifferPage},
    tui::Event,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Home,
    Device,
    Sniffer,
    PacketDetails,
}

pub struct App {
    pub should_quit: bool,
    pub current_page: Page,

    pub home_page: HomePage,
    pub device_page: DevicePage,
    pub sniffer_page: SnifferPage,
    pub packet_details_page: PacketDetailsPage,

    action_tx: mpsc::UnboundedSender<Action>,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        Self {
            should_quit: false,
            current_page: Page::Home,
            home_page: HomePage::new(),
            device_page: DevicePage::new(),
            sniffer_page: SnifferPage::new(),
            packet_details_page: PacketDetailsPage::new(),
            action_tx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let action_tx = self.action_tx.clone();

        // Register action handlers for all components
        self.home_page.register_action_handler(action_tx.clone())?;
        self.device_page
            .register_action_handler(action_tx.clone())?;
        self.sniffer_page
            .register_action_handler(action_tx.clone())?;
        self.packet_details_page
            .register_action_handler(action_tx.clone())?; // Register packet details page

        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn handle_events(&mut self, event: Event) -> Result<()> {
        let action = match event {
            Event::Key(key_event) => {
                if let Some(action) = self.handle_global_key_events(key_event)? {
                    Some(action)
                } else {
                    match self.current_page {
                        Page::Home => self.home_page.handle_events(event)?,
                        Page::Device => self.device_page.handle_events(event)?,
                        Page::Sniffer => self.sniffer_page.handle_events(event)?,
                        Page::PacketDetails => self.packet_details_page.handle_events(event)?, // Handle packet details events
                    }
                }
            }
            Event::Mouse(_) | Event::Tick => match self.current_page {
                Page::Home => self.home_page.handle_events(event)?,
                Page::Device => self.device_page.handle_events(event)?,
                Page::Sniffer => self.sniffer_page.handle_events(event)?,
                Page::PacketDetails => self.packet_details_page.handle_events(event)?, // Handle packet details events
            },
        };

        if let Some(action) = action {
            self.handle_action(action)?;
        }

        Ok(())
    }

    fn handle_global_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                if self.current_page != Page::Home {
                    return Ok(Some(Action::NavigateToHome));
                } else {
                    self.quit();
                    return Ok(None);
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit();
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::NavigateToHome => {
                self.current_page = Page::Home;
            }
            Action::NavigateToDevice => {
                self.current_page = Page::Device;
            }
            Action::NavigateToSniffer => {
                self.current_page = Page::Sniffer;
            }
            Action::DeviceSelected(device_name) => {
                self.sniffer_page
                    .update(Action::DeviceSelected(device_name))?;
                self.current_page = Page::Sniffer;
            }
            Action::PacketSelected(index) => {
                self.sniffer_page.update(Action::PacketSelected(index))?;
                if let Some(packet) = self.sniffer_page.get_packet(index) {
                    self.packet_details_page.set_packet(packet);
                    self.current_page = Page::PacketDetails;
                }
            }
            Action::Quit => {
                self.quit();
            }
            _ => match self.current_page {
                Page::Home => {
                    self.home_page.update(action)?;
                }
                Page::Device => {
                    self.device_page.update(action)?;
                }
                Page::Sniffer => {
                    self.sniffer_page.update(action)?;
                }
                Page::PacketDetails => {
                    // Handle packet details updates
                    self.packet_details_page.update(action)?;
                }
            },
        }
        Ok(())
    }
}

impl ComponentRender<()> for App {
    fn render(&mut self, f: &mut Frame, area: Rect, _props: ()) {
        // Render current page
        match self.current_page {
            Page::Home => self.home_page.render(f, area, ()),
            Page::Device => self.device_page.render(f, area, ()),
            Page::Sniffer => self.sniffer_page.render(f, area, ()),
            Page::PacketDetails => self.packet_details_page.render(f, area, ()), // Render packet details page
        }
    }
}
