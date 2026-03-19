use std::io::stdout;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
};
use futures::{StreamExt, stream::Fuse};
use ratatui::prelude::*;
use ted_config::{Config, ConfigWatcher};
use ted_fs::{FSEvent, Filesystem};
use tokio::sync::mpsc::Receiver;

use crate::{
    state::State,
    widgets::{Filetree, TedWidget},
};

pub struct App {
    state: State,
    filetree: Filetree,

    // Receivers for various state events
    fs_recv: Receiver<FSEvent>,
    config_recv: Receiver<Config>,
    term_recv: Fuse<EventStream>,

    // Watcher handles
    #[allow(dead_code)]
    config_watcher: ConfigWatcher,
}

impl App {
    pub fn new() -> Self {
        let (fs, fs_recv) = Filesystem::new();
        let (config, config_recv, config_watcher) = Config::new();

        Self {
            state: State::new(fs, config),
            filetree: Filetree::new(),
            fs_recv,
            config_recv,
            term_recv: EventStream::new().fuse(),
            config_watcher,
        }
    }

    /// Run the event loop until exit
    pub async fn run(&mut self) -> std::io::Result<()> {
        execute!(stdout(), EnableMouseCapture)?;
        let mut terminal = ratatui::init();
        while !self.state.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events().await;
        }
        ratatui::restore();
        execute!(stdout(), DisableMouseCapture)
    }

    async fn handle_events(&mut self) {
        tokio::select! {
            Some(Ok(event)) = self.term_recv.next() => {
                self.handle_term_event(event);
            }
            Some(event) = self.fs_recv.recv() => {
                self.state.fs.handle_event(event);
            }
            Some(config) = self.config_recv.recv() => {
                self.state.config = config;
            }
        }
    }

    fn handle_term_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') => self.state.exit = true,
                    _ => {}
                };
            }
            _ => {}
        }

        self.filetree.handle(&event, &mut self.state);
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buffer = frame.buffer_mut();

        self.filetree.render(area, buffer, &self.state);
    }
}
