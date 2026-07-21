//! Text highlights for display

use std::ops::Range;

use ropey::Rope;
use tree_sitter::{Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use crate::provider::Provider;

/// A highlight on a single line
pub struct HighlightLine {
    /// Style index
    pub style: usize,
    pub start: usize,
    pub end: usize,
    pub row: usize,
}

// ************************************************************************* //
//                                   ITERATOR                                //
// ************************************************************************* //

/// Iterator over single-line highlights
pub struct HighlightLines {
    style: usize,
    start_col: usize,
    end_col: usize,
    start_row: usize,
    end_row: usize,

    current_row: usize,
}

impl From<QueryCapture<'_>> for HighlightLines {
    fn from(value: QueryCapture) -> Self {
        let node = value.node;
        let start = node.start_position();
        let end = node.end_position();

        Self {
            style: value.index as usize,
            start_col: start.column,
            end_col: end.column,
            start_row: start.row,
            end_row: end.row,
            current_row: start.row,
        }
    }
}

impl Iterator for HighlightLines {
    type Item = HighlightLine;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row > self.end_row {
            return None;
        }

        let is_first = self.current_row == self.start_row;
        let is_last = self.current_row == self.end_row;

        let start = if is_first { self.start_col } else { 0 };
        let end = if is_last { self.end_col } else { usize::MAX };
        let row = self.current_row;

        self.current_row += 1;

        Some(HighlightLine {
            style: self.style,
            start,
            end,
            row,
        })
    }
}

// ************************************************************************* //
//                                 TREE-SITTER                               //
// ************************************************************************* //

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
        let offset = self.rope.line_to_byte(line);
        let line = self.rope.line(line);
        self.range = offset + line.char_to_byte(chars.start)..offset + line.char_to_byte(chars.end);
        self
    }

    pub fn iter(&mut self) -> impl Iterator<Item = HighlightLine> {
        self.cursor.set_byte_range(self.range.clone());
        self.cursor
            .matches(self.query, self.tree.root_node(), Provider(self.rope))
            .map_deref(|m| m.captures.iter().copied())
            .flatten()
            .flat_map(HighlightLines::from)
    }
}
