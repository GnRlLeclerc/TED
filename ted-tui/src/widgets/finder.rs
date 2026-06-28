use crossterm::event::{Event, KeyCode, KeyModifiers};
use hex_color::HexColor;
use nucleo::{Config, Matcher, Utf32Str, Utf32String};
use ratatui::{
    layout::Offset,
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};
use ted_matcher::{
    MatcherView,
    views::{FileView, GrepView},
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

        if let Some(rope) = state.matchers.preview(&state.fs) {
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

        let selected = state.matchers.selected();
        let n_results = results.height.saturating_sub(2) as usize;

        // Adjust the scroll offset to keep the selected item in view
        if self.offset > selected {
            self.offset = selected;
        } else if self.offset + n_results <= selected {
            self.offset = selected.saturating_sub(n_results).saturating_add(1);
        }
        let items = state
            .matchers
            .slice(self.offset, results.height.saturating_sub(2) as usize);

        let padding = results.height.saturating_sub(2 + items.len() as u16);

        Paragraph::new(Text::from(
            (0..padding)
                .map(|_| Line::default())
                .chain(match &items {
                    MatcherView::File(views) => iter_views(
                        &mut self.matcher,
                        &self.filter_utf32,
                        &views,
                        selected.saturating_sub(self.offset),
                        results.width as usize,
                    ),
                    MatcherView::Grep(views) => iter_views(
                        &mut self.matcher,
                        &self.filter_utf32,
                        &views,
                        selected.saturating_sub(self.offset),
                        results.width as usize,
                    ),
                })
                .collect::<Vec<_>>(),
        ))
        .block(Self::block(Color::Blue).title(" Results "))
        .render(results, buf);

        // ***************************************************************** //
        //                               SEARCH                              //
        // ***************************************************************** //

        let matched = state.matchers.matched();
        let total = state.matchers.total();
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
                    state.matchers.close();
                    return Flow::close();
                }
                KeyCode::Enter => {
                    // TODO: run the select option
                    state.matchers.close();
                    return Flow::close();
                }
                KeyCode::Char(char) => {
                    if event.modifiers == KeyModifiers::CONTROL {
                        match char {
                            'j' => {
                                state.matchers.down();
                                state.matchers.ensure_preview(&mut state.fs);
                                return Flow::handled();
                            }
                            'k' => {
                                state.matchers.up();
                                state.matchers.ensure_preview(&mut state.fs);
                                return Flow::handled();
                            }
                            _ => return Flow::not_handled(),
                        }
                    }

                    // Default handling
                    self.filter.push(char);
                    state.matchers.search(&self.filter, true);
                    self.update_filter();
                    return Flow::handled();
                }
                KeyCode::Up => {
                    state.matchers.up();
                    state.matchers.ensure_preview(&mut state.fs);
                    return Flow::handled();
                }
                KeyCode::Down => {
                    state.matchers.down();
                    state.matchers.ensure_preview(&mut state.fs);
                    return Flow::handled();
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    state.matchers.search(&self.filter, false);
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

// ************************************************************************* //
//                                  RENDERING                                //
// ************************************************************************* //

/// Helper trait to render the different results / view kinds from matchers
trait Spans<'a> {
    fn spans(
        &'a self,
        matcher: &mut Matcher,
        filters: &[Utf32String],
        indices: &mut Vec<u32>,
        spans: &mut Vec<Span<'a>>,
    );

    /// Compute matching indices between a haystack and filters
    fn compute_indices(
        haystack: Utf32Str,
        matcher: &mut Matcher,
        filters: &[Utf32String],
        indices: &mut Vec<u32>,
        shift: usize,
    ) {
        indices.clear();
        for filter in filters {
            matcher.fuzzy_indices(haystack, filter.slice(..), indices);
        }
        indices.sort();

        if shift > 0 {
            indices.retain_mut(|i| {
                if *i >= shift as u32 {
                    *i -= shift as u32;
                    true
                } else {
                    false
                }
            });
        }
    }

    /// Split a string into spans based on the provided indices
    fn highlight_indices(s: &'a str, indices: &[u32], spans: &mut Vec<Span<'a>>) {
        let mut chars = s.char_indices().skip(1); // offset by 1 to always fall on the next char boundary
        let mut start_char = 0; // char slice start index
        let mut start_byte = 0; // byte slice start index
        let mut highlighted = true; // highlighted span toggle
        indices
            .chunk_by(|&a, &b| b <= a + 1)
            .flat_map(|chunk| [chunk[0], (chunk[chunk.len() - 1] + 1)])
            .map(|i| {
                let i = i as usize;
                let length = i - start_char;
                start_char = i;
                highlighted = !highlighted;

                (length, highlighted)
            })
            // Filter out possible empty start span
            .filter(|(length, _)| *length > 0)
            .for_each(|(length, highlighted)| {
                let (end_byte, _) = chars.nth(length - 1).unwrap_or((s.len(), char::default()));

                let slice = &s[start_byte..end_byte];
                spans.push(if highlighted {
                    Span::from(slice).blue()
                } else {
                    Span::from(slice)
                });

                start_byte = end_byte;
            });

        // Handle remaining tail
        if start_byte < s.len() {
            spans.push(Span::from(&s[start_byte..]));
        }
    }
}

impl<'a, 'inner> Spans<'a> for FileView<'inner>
where
    'inner: 'a,
{
    fn spans(
        &'a self,
        matcher: &mut Matcher,
        filters: &[Utf32String],
        indices: &mut Vec<u32>,
        spans: &mut Vec<Span<'a>>,
    ) {
        // Add file icon
        let color = HexColor::parse(self.icon.color).unwrap_or_default();
        spans.push(
            Span::raw(format!("{} ", self.icon.icon)).fg(Color::Rgb(color.r, color.g, color.b)),
        );

        // Compute indices with shift=2 to remove leading ./ from paths
        Self::compute_indices(self.utf32, matcher, filters, indices, 2);
        Self::highlight_indices(&self.string[2..], indices, spans);
    }
}

impl<'a, 'inner> Spans<'a> for GrepView<'inner>
where
    'inner: 'a,
{
    fn spans(
        &'a self,
        _matcher: &mut Matcher,
        _filters: &[Utf32String],
        _indices: &mut Vec<u32>,
        spans: &mut Vec<Span<'a>>,
    ) {
        // Add file icon
        let color = HexColor::parse(self.icon.color).unwrap_or_default();
        spans.push(
            Span::raw(format!("{} ", self.icon.icon)).fg(Color::Rgb(color.r, color.g, color.b)),
        );

        // Add raw line (matching is not done on the file names)
        let s: &str = &self.string;
        spans.push(Span::from(&s[2..]));

        // Add the line number
        spans.push(Span::from(":"));
        spans.push(Span::from(self.line.to_string()).red());
    }
}

/// Produce an iterator of lines from the views
/// The iterator is boxed to erase the input type.
fn iter_views<'a, 'b: 'a, T: Spans<'a>>(
    matcher: &'b mut Matcher,
    filters: &'b [Utf32String],
    views: &'a [T],
    selected: usize,
    width: usize,
) -> Box<dyn Iterator<Item = Line<'a>> + 'a> {
    let mut indices = vec![];
    Box::new(views.iter().enumerate().rev().map(move |(i, view)| {
        let selected = i == selected;
        let mut spans = vec![Span::from(if selected { "> " } else { "  " })];
        view.spans(matcher, filters, &mut indices, &mut spans);

        let mut line = Line::from(spans);

        if selected {
            // Prolongate the line so that the background color covers 100% width
            line.push_span(Span::from(" ".repeat((width).saturating_sub(line.width()))));
            line = line.on_dark_gray().bold();
        }
        line
    }))
}
