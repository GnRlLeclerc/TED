use ratatui::{prelude::*, widgets::Paragraph};
use ted_buffer::Buffer as TextBuffer;

pub struct FileBuffer<'a> {
    text: &'a TextBuffer,
    /// Optional highlighted line
    line: Option<usize>,
    scroll_x: usize,
    scroll_y: usize,
}

impl<'a> FileBuffer<'a> {
    pub fn preview(text: &'a TextBuffer, line: Option<usize>) -> Self {
        Self {
            text,
            line,
            scroll_x: 0,
            scroll_y: 0,
        }
    }

    fn raw(&self, area: Rect) -> Paragraph<'a> {
        Paragraph::new(Text::from(
            self.text
                .rope
                .lines_at(self.scroll_y)
                .take(area.height as usize)
                .map(|line| {
                    let len = line.len_chars();
                    Line::from_iter(
                        line.slice(
                            self.scroll_x.min(len)..(self.scroll_x + area.width as usize).min(len),
                        )
                        .chunks()
                        .map(|chunk| Span::raw(chunk)),
                    )
                })
                .collect::<Vec<_>>(),
        ))
    }
}

impl<'a> Widget for FileBuffer<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // Scroll to put the highlighted line in the middle of the screen.
        if let Some(line) = self.line {
            self.scroll_y = line.saturating_sub(area.height as usize / 2);
        }

        self.raw(area).render(area, buf);

        if let Some(line) = self.line
            && line >= self.scroll_y
            && line < self.scroll_y + area.height as usize
        {
            let rect = Rect::new(
                area.x,
                area.y + (line - self.scroll_y) as u16,
                self.text
                    .rope
                    .line(line)
                    .len_chars()
                    .saturating_sub(self.scroll_x)
                    .min(area.width as usize) as u16,
                1,
            );
            buf.set_style(rect, Style::default().on_dark_gray());
        }
    }
}
