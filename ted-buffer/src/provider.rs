//! Ropey text provider for tree-sitter highlighting queries.

use ropey::{Rope, iter::Chunks};
use tree_sitter::{Node, TextProvider};

use crate::utils::{char_end, char_start};

/// Text provider wrapper for ropey buffer in order to compute
/// highlighting queries.
pub struct Provider<'a>(pub &'a Rope);

impl<'a> TextProvider<&'a [u8]> for Provider<'a> {
    type I = ByteChunks<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let start = char_start(self.0, node);
        let end = char_end(self.0, node);

        let slice = self.0.slice(start..end);
        ByteChunks(slice.chunks())
    }
}

/// Chunks iterator struct for the text provider I type
/// (cannot use Map<Chunks, impl FnMut _> because impl is disallowed in type aliases).
pub struct ByteChunks<'b>(Chunks<'b>);

impl<'b> Iterator for ByteChunks<'b> {
    type Item = &'b [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(str::as_bytes)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
