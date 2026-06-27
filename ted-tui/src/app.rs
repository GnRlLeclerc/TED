use std::{io::stdout, path::Path};

use crossterm::{
    cursor::SetCursorStyle,
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
};
use futures::{StreamExt, stream::Fuse};
use ratatui::prelude::*;
use ted_config::{Config, ConfigWatcher};
use ted_fs::{FSEvent, Filesystem};
use ted_matcher::{MatcherData, Matchers};
use tokio::sync::watch::Receiver as WatchReceiver;
use tokio::{sync::mpsc::Receiver, time::Instant};

use crate::{
    layouts::{Drawers, Panes},
    state::State,
    utils::Side,
    widgets::{ClonableWidget, Filetree, Finder, Flow, FlowExt, Handled, Home, TedWidget},
};

pub struct App {
    state: State,

    /// Editor screen
    editor: Drawers<Panes>,

    // Receivers for various state events
    fs_recv: Receiver<FSEvent>,
    config_recv: Receiver<Config>,
    term_recv: Fuse<EventStream>,
    matcher_recv: WatchReceiver<Instant>,

    // Watcher handles
    #[allow(dead_code)]
    config_watcher: ConfigWatcher,
}

impl App {
    pub fn new() -> Self {
        let (fs, fs_recv) = Filesystem::new();
        let (config, config_recv, config_watcher) = Config::new();
        let (matchers, matcher_recv) = Matchers::new();

        let mut state = State::new(fs, config, matchers);

        Self {
            state,
            editor: drawers,

            fs_recv,
            config_recv,
            term_recv: EventStream::new().fuse(),
            matcher_recv,
            config_watcher,
        }
    }

    /// Run the event loop until exit
    pub async fn run(&mut self) -> std::io::Result<()> {
        execute!(stdout(), EnableMouseCapture)?;
        let mut terminal = ratatui::init();
        execute!(stdout(), SetCursorStyle::SteadyBlock)?;

        // Initial rendering
        terminal.draw(|frame| self.render(frame))?;

        loop {
            tokio::select! {
                biased; // prioritize terminal events

                Some(Ok(event)) = self.term_recv.next() => {
                    match self.handle_term_event(event) {
                        Flow::Continue(_) => continue,
                        Flow::Break(Handled::Close) => break,
                        _ => {}
                    }
                }
                Some(event) = self.fs_recv.recv() => {
                    let notify_matcher = matches!(
                        event,
                        FSEvent::BufferLoaded { .. } | FSEvent::OrphanLoaded { .. }
                    );
                    self.state.fs.handle_event(event);
                    if notify_matcher {
                        self.state.matchers.ensure_preview(&mut self.state.fs);
                    }
                }
                Some(config) = self.config_recv.recv() => {
                    self.state.config = config;
                }
                Ok(_) = self.matcher_recv.changed() => {
                    let instant = *self.matcher_recv.borrow();
                    if !self.state.matchers.tick(instant) {
                        continue;
                    }
                    self.state.matchers.ensure_preview(&mut self.state.fs);
                }
            }
            terminal.draw(|frame| self.render(frame))?;
        }

        ratatui::restore();
        execute!(stdout(), DisableMouseCapture)
    }

    fn handle_term_event(&mut self, event: Event) -> Flow {
        if matches!(event, Event::Resize(_, _)) {
            return Flow::handled();
        }

        self.editor.handle(&event, &mut self.state)?;

        if let Event::Key(key) = event {
            if key.code == KeyCode::Char('f') && key.modifiers.is_empty() {
                self.editor.floating(Finder::new().boxed());
                self.state.matchers.open(MatcherData::File(Path::new(".")));
                return Flow::handled();
            }
        }

        Flow::not_handled()
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buffer = frame.buffer_mut();

        self.editor.render(area, buffer, &self.state);

        self.state.cursor = self.editor.cursor(&self.state);
        frame.set_cursor_position(self.state.cursor);
    }
}
