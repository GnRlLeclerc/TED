//! Text highlights for display

use std::ops::Range;

use ropey::Rope;
use streaming_iterator::convert;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

use crate::{
    provider::Provider,
    utils::{char_end, char_start},
};

/// Highlight range, for writing into ratatui
pub struct Highlight {
    /// Start char index
    pub start: usize,
    /// End char index
    pub end: usize,
    /// Style index in the query capture names slice
    pub style: usize,
}

/// Highlight computation from tree-sitter
pub struct HighlightsTS<'a> {
    range: Range<usize>,
    cursor: QueryCursor,
    tree: &'a Tree,
    query: &'a Query,
    rope: &'a Rope,
}

impl<'a> HighlightsTS<'a> {
    pub fn new(tree: &'a Tree, query: &'a Query, rope: &'a Rope) -> Self {
        let range = 0..rope.len_bytes();
        let cursor = QueryCursor::new();
        Self {
            range,
            cursor,
            tree,
            query,
            rope,
        }
    }

    /// Set the byte range for a single line
    pub fn line(mut self, line: usize) -> Self {
        let start = self.rope.line_to_byte(line);
        let end = start + self.rope.line(line).len_bytes();

        self.range = start..end;
        self
    }

    /// Set the byte range for a range of lines
    pub fn lines(mut self, lines: Range<usize>) -> Self {
        let start = self.rope.line_to_byte(lines.start);
        let end = match lines.end >= self.rope.len_lines() {
            true => self.rope.len_bytes(),
            false => self.rope.line_to_byte(lines.end),
        };

        self.range = start..end;
        self
    }

    /// Set the byte range for a range of chars within a line
    pub fn line_range(mut self, line: usize, chars: Range<usize>) -> Self {
        let line = self.rope.line(line);
        self.range = line.char_to_byte(chars.start)..line.char_to_byte(chars.end);
        self
    }

    pub fn iter(&mut self) -> impl StreamingIterator<Item = Highlight> {
        self.cursor.set_byte_range(self.range.clone());
        self.cursor
            .matches(self.query, self.tree.root_node(), Provider(self.rope))
            .flat_map(|m| {
                convert(m.captures.iter().map(|capture| {
                    let node = capture.node;
                    Highlight {
                        start: char_start(self.rope, node),
                        end: char_end(self.rope, node),
                        style: capture.index as usize,
                    }
                }))
            })
    }
}
