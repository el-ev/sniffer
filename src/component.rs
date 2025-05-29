use anyhow::Result;
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc;

use crate::{action::Action, tui::Event};

pub trait Component {
    fn register_action_handler(&mut self, tx: mpsc::UnboundedSender<Action>) -> Result<()>;
    fn handle_events(&mut self, event: Event) -> Result<Option<Action>>;
    fn handle_key_events(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>>;
    fn update(&mut self, action: Action) -> Result<Option<Action>>;
}

pub trait ComponentRender<Props> {
    fn render(&mut self, f: &mut Frame, area: Rect, props: Props);
}
