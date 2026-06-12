use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::Offset,
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

use crate::{
    state::State,
    widgets::{Flow, FlowExt, TedWidget},
};

/// Finder widget
pub struct Finder {
    area: Rect,
    cursor: Position,
    filter: String,
}

impl Finder {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            cursor: Position::default(),
            filter: "".to_string(),
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

        let filter_len = search.width as usize - 4;
        let offset = self.filter.chars().count().saturating_sub(filter_len);
        self.cursor = search.as_position()
            + Offset::new(2 + self.filter.chars().count().min(filter_len) as i32, 1);

        Clear.render(preview, buf);
        Clear.render(results, buf);
        Clear.render(search, buf);

        Self::block(Color::Green)
            .title(" Preview ")
            .render(preview, buf);
        Self::block(Color::Blue)
            .title(" Results ")
            .render(results, buf);
        Paragraph::new(&self.filter[offset..])
            .block(
                Self::block(Color::Red)
                    .title(" Find Files ")
                    .padding(Padding::horizontal(1)),
            )
            .render(search, buf);
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> Flow {
        match event {
            Event::Key(event) => match event.code {
                KeyCode::Esc => return Flow::close(),
                KeyCode::Char(char) => {
                    self.filter.push(char);
                    return Flow::handled();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    return Flow::handled();
                }
                _ => {}
            },

            _ => {}
        }

        Flow::not_handled()
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn cursor(&self, _: &State) -> Position {
        self.cursor
    }
}
