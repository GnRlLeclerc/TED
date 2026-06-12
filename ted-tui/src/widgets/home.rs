use crossterm::event::{Event, KeyCode};
use ratatui::prelude::*;

use crate::{
    state::State,
    widgets::{Flow, FlowExt, TedWidget},
};

pub struct Home {
    title: String,
    area: Rect,
}

impl Home {
    pub fn new() -> Self {
        Self {
            title: "TED".to_string(),
            area: Rect::default(),
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
        self.area = area;
    }

    fn handle(&mut self, event: &Event, _: &mut State) -> Flow {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return Flow::close(),
                _ => {}
            },
            _ => {}
        }

        Flow::not_handled()
    }

    fn area(&self) -> Rect {
        self.area
    }
}
