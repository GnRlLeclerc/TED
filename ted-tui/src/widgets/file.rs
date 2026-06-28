use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph},
};
use ropey::Rope;

/// File buffer display widget
pub struct FileBuffer<'a> {
    /// Optional highlighted line
    line: Option<usize>,
    rope: &'a Rope,
    scroll_y: usize,
    block: Option<Block<'a>>,
}

impl<'a> FileBuffer<'a> {
    pub fn new(rope: &'a Rope, scroll_y: usize, line: Option<usize>) -> Self {
        Self {
            line,
            rope,
            scroll_y,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> Widget for FileBuffer<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let buffer = self.rope;

        // Scroll to put the highlighted line in the middle of the screen.
        if let Some(line) = self.line {
            self.scroll_y = line.saturating_sub(area.height as usize / 2 - 1);
        }

        let mut par = Paragraph::new(Text::from(
            (self.scroll_y..buffer.len_lines().min(area.height as usize + self.scroll_y))
                .map(|line_idx| {
                    let mut remaining = area.width as usize;
                    let slice = buffer.line(line_idx);
                    let mut line = Line::from_iter(slice.chunks().map_while(|chunk| {
                        if remaining == 0 {
                            return None;
                        }

                        let n = chunk.chars().count().min(remaining);
                        remaining -= n;

                        let byte_end = chunk
                            .char_indices()
                            .nth(n)
                            .map(|(idx, _)| idx)
                            .unwrap_or(chunk.len());
                        Some(&chunk[..byte_end])
                    }));

                    if matches!(self.line, Some(i) if i == line_idx + 1) {
                        line = line.on_dark_gray();
                    }

                    line
                })
                .collect::<Vec<_>>(),
        ));

        if let Some(block) = self.block {
            par = par.block(block);
        }

        par.render(area, buf);
    }
}
