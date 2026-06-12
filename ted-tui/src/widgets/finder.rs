use crossterm::event::Event;
use ratatui::{
    layout::Offset,
    prelude::*,
    widgets::{Block, BorderType, Clear},
};

use crate::{
    state::State,
    widgets::{Flow, FlowExt, TedWidget},
};

/// Finder widget
pub struct Finder {
    area: Rect,
    cursor: Position,
}

impl Finder {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            cursor: Position::default(),
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

        self.cursor = search.as_position() + Offset::new(2, 1);

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
        Flow::close()
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn cursor(&self, _: &State) -> Position {
        self.cursor
    }
}
