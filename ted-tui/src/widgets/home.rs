use crossterm::event::{Event, KeyCode};
use ratatui::prelude::*;

use crate::{state::State, widgets::TedWidget};

pub struct Home {
    title: String,
}

impl Home {
    pub fn new() -> Self {
        Self {
            title: "TED".to_string(),
        }
    }
}

impl TedWidget for Home {
    fn render(&mut self, area: Rect, buf: &mut Buffer, _: &State) {
        let [_, line, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);
        Line::from(self.title.as_str()).centered().render(line, buf);
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => state.exit = true,
                _ => return false,
            },
            _ => return false,
        }

        true
    }
}
