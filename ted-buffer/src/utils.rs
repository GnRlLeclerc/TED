use ropey::Rope;
use tree_sitter::Node;

pub fn char_start(rope: &Rope, node: Node) -> usize {
    let start = node.start_position();
    rope.line_to_char(start.row) + start.column
}

pub fn char_end(rope: &Rope, node: Node) -> usize {
    let end = node.end_position();
    rope.line_to_char(end.row) + end.column
}
