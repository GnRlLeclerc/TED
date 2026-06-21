use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph},
};
use ropey::Rope;

/// File buffer display widget
pub struct FileBuffer<'a> {
    rope: &'a Rope,
    scroll_y: usize,
    block: Option<Block<'a>>,
}

impl<'a> FileBuffer<'a> {
    pub fn new(rope: &'a Rope, scroll_y: usize) -> Self {
        Self {
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
    fn render(self, area: Rect, buf: &mut Buffer) {
        let buffer = self.rope;

        let mut par = Paragraph::new(Text::from(
            (self.scroll_y..buffer.len_lines().min(area.height as usize + self.scroll_y))
                .map(|line| {
                    let mut remaining = area.width as usize;
                    let line = buffer.line(line);
                    Line::from_iter(line.chunks().map_while(|chunk| {
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
                    }))
                })
                .collect::<Vec<_>>(),
        ));

        if let Some(block) = self.block {
            par = par.block(block);
        }

        par.render(area, buf);
    }
}
