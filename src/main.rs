use anyhow::Result;
use component::ComponentRender;
use ratatui::crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use tokio::time::{self, Duration};

mod action;
mod app;
mod component;
mod pages;
mod tui;
mod data;

use app::App;
use tui::{Event, Tui};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().map_err(|_| anyhow::anyhow!("Failed to install color_eyre"))?;

    let mut tui = Tui::new()?;
    tui.enter()?;

    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut app = App::new(action_tx);
    app.run().await?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let ticker_tx = tx.clone();

    tokio::spawn(async move {
        let mut ticker = time::interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;
            if ticker_tx.send(Event::Tick).is_err() {
                break;
            }
        }
    });

    loop {
        let timeout = Duration::from_millis(16); // ~60 FPS

        if event::poll(timeout)? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        app.handle_events(Event::Key(key))?;
                    }
                }
                CrosstermEvent::Mouse(mouse) => {
                    app.handle_events(Event::Mouse(mouse))?;
                }
                _ => {}
            }
        }

        if let Ok(action) = action_rx.try_recv() {
            app.handle_action(action)?;
        }

        if let Some(e) = rx.recv().await {
            app.handle_events(e)?;
        }

        if app.should_quit {
            break;
        }

        tui.draw(|f| {
            app.render(f, f.area(), ());
        })?;
    }

    tui.exit()?;
    Ok(())
}
