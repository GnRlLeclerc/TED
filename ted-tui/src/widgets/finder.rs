use crossterm::event::{Event, KeyCode, KeyModifiers};
use nucleo::{Config, Matcher, Utf32String};
use ratatui::{
    layout::Offset,
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};

use crate::{
    state::State,
    widgets::{FileBuffer, Flow, FlowExt, TedWidget},
};

/// Finder widget
pub struct Finder {
    area: Rect,
    cursor: Position,
    matcher: Matcher,
    filter: String,
    /// Filter whitespace-delimited substrings for highlighting
    filter_utf32: Vec<Utf32String>,
    /// Scroll offset
    offset: usize,
}

impl Finder {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            cursor: Position::default(),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            filter: "".to_string(),
            filter_utf32: Vec::new(),
            offset: 0,
        }
    }

    fn block(text_bg: Color) -> Block<'static> {
        Block::bordered()
            .title_alignment(HorizontalAlignment::Center)
            .title_style(Style::default().black().bg(text_bg))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().dark_gray())
    }

    fn update_filter(&mut self) {
        self.filter_utf32 = self
            .filter
            .split_whitespace()
            .into_iter()
            .map(Utf32String::from)
            .collect();
        self.offset = 0;
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
        self.area = main;

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

        // ***************************************************************** //
        //                               PREVIEW                             //
        // ***************************************************************** //

        if let Some(rope) = state.matcher.preview(&state.fs) {
            FileBuffer::new(rope, 0)
                .block(Self::block(Color::Green).title(" Preview "))
                .render(preview, buf);
        } else {
            Self::block(Color::Green)
                .title(" Preview ")
                .render(preview, buf);
        }

        // ***************************************************************** //
        //                               RESULTS                             //
        // ***************************************************************** //

        let selected = state.matcher.selected();
        let n_results = results.height.saturating_sub(2) as usize;

        // Adjust the scroll offset to keep the selected item in view
        if self.offset > selected {
            self.offset = selected;
        } else if self.offset + n_results <= selected {
            self.offset = selected.saturating_sub(n_results).saturating_add(1);
        }
        let items = state
            .matcher
            .slice(self.offset as u32, results.height.saturating_sub(2) as u32);

        let padding = results.height.saturating_sub(2 + items.len() as u16);
        let mut indices = vec![];

        Paragraph::new(Text::from(
            (0..padding)
                .map(|_| Line::default())
                .chain(items.iter().enumerate().rev().map(|(i, item)| {
                    indices.clear();
                    for filter in &self.filter_utf32 {
                        self.matcher
                            .fuzzy_indices(item.utf32, filter.slice(..), &mut indices);
                    }
                    indices.sort();

                    let selected = i + self.offset == selected;
                    let mut start: usize = 0;
                    let mut spans = vec![Span::from(if selected { "> " } else { "  " })];
                    let s: &str = &item.string;
                    let s = &s[2..];

                    indices.retain_mut(|i| {
                        if *i >= 2 {
                            *i -= 2;
                            true
                        } else {
                            false
                        }
                    });

                    indices.chunk_by(|&a, &b| b <= a + 1).for_each(|chunk| {
                        let chunk_0 = chunk[0] as usize;

                        // 1. Pre-chunk
                        if start < chunk_0 {
                            spans.push(Span::from(&s[start..chunk_0]));
                        }
                        // 2. Chunk
                        let end = (chunk[chunk.len() - 1] + 1) as usize;
                        spans.push(s[chunk_0..end].blue());
                        start = end;
                    });

                    // 3. Post-chunks
                    if start < s.len() {
                        spans.push(Span::from(&s[start..]));
                    }

                    let mut line = Line::from(spans);

                    if selected {
                        // Prolongate the line so that the background color covers 100% width
                        line.push_span(Span::from(
                            " ".repeat((results.width as usize).saturating_sub(line.width())),
                        ));
                        line = line.on_dark_gray().bold();
                    }

                    line
                }))
                .collect::<Vec<_>>(),
        ))
        .block(Self::block(Color::Blue).title(" Results "))
        .render(results, buf);

        // ***************************************************************** //
        //                               SEARCH                              //
        // ***************************************************************** //

        let matched = state.matcher.matched();
        let total = state.matcher.total();
        let available = search.width.saturating_sub(4) as usize;
        let counter_cols = 4 + cols(total) + cols(matched);
        let filter_cols = self.filter.chars().count();
        let filter_len = available.saturating_sub(counter_cols);
        let offset = filter_cols.saturating_sub(filter_len);
        let padding = available.saturating_sub(filter_cols + counter_cols);

        Paragraph::new(Text::from(Line::from(vec![
            Span::from(&self.filter[offset..]),
            Span::from(" ".repeat(padding as usize)),
            Span::from(format!(" {} / {}", matched, total).dark_gray()),
        ])))
        .block(
            Self::block(Color::Red)
                .title(" Find Files ")
                .padding(Padding::horizontal(1)),
        )
        .render(search, buf);

        self.cursor = search.as_position() + Offset::new(2 + filter_cols.min(filter_len) as i32, 1);
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> Flow {
        match event {
            Event::Key(event) => match event.code {
                KeyCode::Esc => {
                    state.matcher.close();
                    return Flow::close();
                }
                KeyCode::Enter => {
                    // TODO: run the select option
                    state.matcher.close();
                    return Flow::close();
                }
                KeyCode::Char(char) => {
                    if event.modifiers == KeyModifiers::CONTROL {
                        match char {
                            'j' => {
                                state.matcher.down(&mut state.fs);
                                return Flow::handled();
                            }
                            'k' => {
                                state.matcher.up(&mut state.fs);
                                return Flow::handled();
                            }
                            _ => return Flow::not_handled(),
                        }
                    }

                    // Default handling
                    self.filter.push(char);
                    state.matcher.search(&self.filter, true);
                    self.update_filter();
                    return Flow::handled();
                }
                KeyCode::Up => {
                    state.matcher.up(&mut state.fs);
                    return Flow::handled();
                }
                KeyCode::Down => {
                    state.matcher.down(&mut state.fs);
                    return Flow::handled();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    state.matcher.search(&self.filter, false);
                    self.update_filter();
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

/// Returns the amount of columns needed to display the number n
fn cols(n: usize) -> usize {
    if n == 0 {
        return 1;
    }

    n.ilog10() as usize + 1
}
