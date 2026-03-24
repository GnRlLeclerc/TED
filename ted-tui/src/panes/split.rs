use super::Child;
use ratatui::prelude::*;

use crate::{
    panes::{PaneKey, SplitKey},
    utils::drag_to_cursor,
};

/// A single split in a split layout
pub struct Split {
    pub area: Rect,
    pub direction: Direction,
    pub children: Vec<Child>,
    pub parent: Option<SplitKey>,
    /// Child pane sizes along the split direction,
    /// used to compute the layout for the next frame
    /// and for click collisions.
    pub sizes: Vec<u16>,
}

impl Split {
    /// Create a new child split along a direction, with 2 child panes.
    pub fn new(direction: Direction, parent: SplitKey, panes: [PaneKey; 2]) -> Self {
        Self {
            direction,
            children: vec![Child::Pane(panes[0]), Child::Pane(panes[1])],
            parent: Some(parent),
            sizes: vec![1, 1],
            area: Rect::default(),
        }
    }

    pub fn drag_to_cursor(&mut self, border: usize, cursor: Position) {
        drag_to_cursor(&mut self.sizes, border, cursor, self.direction, self.area);
    }

    /// Remove the n-th child
    pub fn remove(&mut self, child: usize) {
        self.sizes.remove(child);
        self.children.remove(child);
    }
}
