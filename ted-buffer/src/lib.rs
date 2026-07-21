use ropey::Rope;
use tree_sitter::{Parser, Query, Tree};

mod highlights;
mod provider;
mod utils;

pub use highlights::{HighlightLine, HighlightLines, HighlightsTS};

/// A text buffer with optional highlighting
pub struct Buffer {
    pub rope: Rope,
    pub tree: Option<Tree>,
    pub parser: Option<Parser>,
}

impl Buffer {
    pub fn new(string: &str) -> Self {
        Self {
            rope: Rope::from_str(string),
            tree: None,
            parser: None,
        }
    }

    /// Initial full parsing of the buffer content
    pub fn parse(&mut self) {
        self.tree = self
            .parser
            .as_mut()
            .map(|parser| {
                // Parse with options to feed treesitter continuous slices of bytes
                // from the ropey buffer.
                parser.parse_with_options(
                    &mut |offset, _| {
                        if offset >= self.rope.len_bytes() {
                            return &[] as &[u8];
                        }
                        let (chunk, chunk_start, _, _) = self.rope.chunk_at_byte(offset);
                        &chunk.as_bytes()[offset - chunk_start..]
                    },
                    None,
                    None,
                )
            })
            .unwrap();
    }

    /// TODO: after text editing, update the tree with incremental parsing
    pub fn edit(&mut self) {}

    /// Return a highlight iterator for the given query on the buffer
    pub fn highlights_ts<'a>(&'a self, query: &'a Query) -> Option<HighlightsTS<'a>> {
        self.tree
            .as_ref()
            .map(|tree| HighlightsTS::new(tree, query, &self.rope))
    }
}
