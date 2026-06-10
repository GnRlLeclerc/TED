use crossterm::event::Event;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Clear},
};

use crate::{
    state::State,
    widgets::{Flow, TedWidget},
};

/// Finder widget
pub struct Finder {
    area: Rect,
}

impl Finder {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
        }
    }

    fn block(text_bg: Color) -> Block<'static> {
        Block::bordered()
            .title_alignment(HorizontalAlignment::Center)
            .title_style(Style::default().black().bg(text_bg))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().dark_gray())
    }
}

impl TedWidget for Finder {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let [_, main, _] = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .areas(area);

        let [_, preview, results, search, _] = Layout::vertical([
            Constraint::Percentage(5),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Percentage(10),
        ])
        .areas(main);

        Clear.render(preview, buf);
        Clear.render(results, buf);
        Clear.render(search, buf);

        Self::block(Color::Green)
            .title(" Preview ")
            .render(preview, buf);
        Self::block(Color::Blue)
            .title(" Results ")
            .render(results, buf);
        Self::block(Color::Red)
            .title(" Find Files ")
            .render(search, buf);
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> Flow {
        Flow::Close
    }

    fn area(&self) -> Rect {
        self.area
    }
}
