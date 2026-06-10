use std::iter::once;

use crate::{
    state::State,
    utils::{Side, drag_to_cursor},
    widgets::{Border, ClonableWidget, Flow, TedWidget},
};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::prelude::*;
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    struct SplitKey;
    // DEBUG: set to pub for creating dummy panes
    pub struct PaneKey;
}

#[derive(Clone, Copy)]
enum Child {
    Pane(PaneKey),
    Split(SplitKey),
}

pub struct Panes {
    /// The root is always horizontal split
    root: SplitKey,
    /// Default widget being displayed when the root split is empty
    default: Box<dyn TedWidget>,
    drag: Option<(SplitKey, usize)>,
    splits: SlotMap<SplitKey, Split>,
    panes: SlotMap<PaneKey, (Box<dyn ClonableWidget>, SplitKey)>,
    // DEBUG: pub for pane creation
    pub focused: Option<PaneKey>,
}

// ************************************************************************* //
//                                PUBLIC API                                 //
// ************************************************************************* //

impl Panes {
    /// Create a new empty panes layout with no inner panes
    pub fn new(default: Box<dyn TedWidget>) -> Self {
        let mut splits = SlotMap::with_key();
        let root = splits.insert(Split {
            direction: Direction::Horizontal,
            children: Vec::new(),
            parent: None,
            sizes: Vec::new(),
            area: Rect::default(),
        });

        Self {
            root,
            default,
            drag: None,
            splits,
            panes: SlotMap::with_key(),
            focused: None,
        }
    }

    /// Open a widget in the focused pane.
    /// If there is no focused pane, open it in the root split.
    /// Normally, there should always be a focused pane,
    /// except when the root split is empty.
    pub fn open(&mut self, widget: Box<dyn ClonableWidget>, state: &mut State) {
        if let Some(focused) = self.focused {
            // Replace the focused pane
            self.panes[focused].0.close();
            self.panes[focused].0 = widget;
        } else {
            let key = self.panes.insert((widget, self.root));
            self.splits[self.root].children.push(Child::Pane(key));
            self.splits[self.root].sizes.push(1);
            self.focus(key, state);
        }
    }

    /// Split a pane along the given direction,
    /// cloning its inner widget.
    pub fn split(&mut self, key: PaneKey, state: &mut State, direction: Direction) {
        let (parent, pane_index) = self.pane_parent(key);
        let clone = self.panes[key].0.clone();
        let clone_key = self.panes.insert((clone, parent));

        if self.splits[parent].direction == direction {
            let width = self.splits[parent].sizes[pane_index];

            self.splits[parent]
                .children
                .insert(pane_index, Child::Pane(clone_key));
            self.splits[parent].sizes.insert(pane_index, width);
        } else {
            // Replace the pane with a split that contains it twice
            let split = Split::new(direction, parent, [key, clone_key]);
            let parent_key = self.splits.insert(split);
            self.splits[parent].children[pane_index] = Child::Split(parent_key);
            self.panes[key].1 = parent_key;
            self.panes[clone_key].1 = parent_key;
        }

        self.focus(clone_key, state);
    }

    /// Close a pane, closing its parent split if it has only 2 children
    /// (i.e. itself and a sibling)
    pub fn close(&mut self, key: PaneKey, state: &mut State) {
        let (parent, pane_index) = self.pane_parent(key);
        let change_focus = matches!(self.focused, Some(focused) if focused == key);
        self.panes.remove(key).map(|(w, _)| w.close());

        if self.splits[parent].children.len() > 2 || self.splits[parent].parent.is_none() {
            // > 2 children or root split: just remove the pane from the split
            self.splits[parent].remove(pane_index);
            if change_focus {
                self.focus_nearest_sibling(parent, state, pane_index);
            }
        } else if let Some(split_parent) = self.splits[parent].parent {
            // Not the root split and exactly 2 children,
            // remove the split and replace it with the remaining sibling child
            let remaining = self.splits[parent].children[1 - pane_index];
            self.splits.remove(parent);

            let index = self.splits[split_parent]
                .children
                .iter()
                .position(|child| matches!(child, Child::Split(k) if *k == parent))
                .unwrap();
            self.splits[split_parent].children[index] = remaining;

            // Update parent key in the remaining child
            match remaining {
                Child::Pane(pane) => self.panes[pane].1 = split_parent,
                Child::Split(child) => self.splits[child].parent = Some(split_parent),
            }

            if change_focus {
                self.focus_nearest_sibling(split_parent, state, index);
            }
        }
    }
}

// ************************************************************************* //
//                                    SPLIT                                  //
// ************************************************************************* //

/// A single split in a split layout
struct Split {
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

// ************************************************************************* //
//                                   WIDGET                                  //
// ************************************************************************* //

impl TedWidget for Panes {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        if self.splits[self.root].children.is_empty() {
            self.default.render(area, buf, state);
        } else {
            self.render_split(self.root, area, buf, state);
        }
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> Flow {
        // Use a flag to allow handling the event multiple times in the specific case
        // where a pane is clicked and its content is then also allowed to handle the click event.
        let mut handled = Flow::NotHandled;

        match event {
            Event::Mouse(mouse) => {
                let cursor = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(_) => {
                        if let Some(hit) = self.recurse_click(cursor, self.root) {
                            match hit {
                                ClickResult::Pane(pane) => {
                                    self.focused = Some(pane);
                                    self.drag = None;
                                    handled = Flow::Handled;
                                }
                                ClickResult::Border(split, border) => {
                                    self.drag = Some((split, border));
                                    // Started dragging, stop propagation
                                    return Flow::Handled;
                                }
                            }
                        }
                    }
                    MouseEventKind::Up(_) => {
                        self.drag = None;
                    }
                    MouseEventKind::Drag(_) => {
                        if let Some((split, border)) = self.drag {
                            self.splits[split].drag_to_cursor(border, cursor);
                            return Flow::Handled;
                        }
                    }
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        return self.recurse_scroll(cursor, self.root, event, state);
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if self.splits[self.root].children.is_empty() {
            return self.default.handle(event, state);
        }

        // Propagate to the focused pane
        if let Some(focused) = self.focused {
            match self.panes[focused].0.handle(event, state) {
                Flow::Handled => return Flow::Handled,
                Flow::Close => {
                    self.close(focused, state);
                    return Flow::Handled;
                }
                Flow::NotHandled => {}
            }
        }

        // Move inbetween panes
        if let Event::Key(key_event) = event
            && key_event.modifiers == KeyModifiers::CONTROL
            && let Some(key) = self.focused
        {
            if let Some(target) = match key_event.code {
                KeyCode::Char('h') => self.focus_neighbor(key, Side::Left, state.cursor),
                KeyCode::Char('j') => self.focus_neighbor(key, Side::Bottom, state.cursor),
                KeyCode::Char('k') => self.focus_neighbor(key, Side::Top, state.cursor),
                KeyCode::Char('l') => self.focus_neighbor(key, Side::Right, state.cursor),
                _ => None,
            } {
                self.focus(target, state);
                return Flow::Handled;
            }
        }

        handled
    }

    fn cursor(&self, state: &State) -> Position {
        match self.focused {
            Some(focused) => self.panes[focused].0.cursor(state),
            None => self.default.cursor(state),
        }
    }

    fn area(&self) -> Rect {
        self.splits[self.root].area
    }
}

// ************************************************************************* //
//                               EVENT HANDLING                              //
// ************************************************************************* //

enum ClickResult {
    Pane(PaneKey),
    Border(SplitKey, usize),
}

impl Panes {
    /// Returns the parent key of a pane, and the pane index in the parent's children
    fn pane_parent(&self, key: PaneKey) -> (SplitKey, usize) {
        let parent = self.panes[key].1;
        let index = self.splits[parent]
            .children
            .iter()
            .position(|child| matches!(child, Child::Pane(k) if *k == key))
            .unwrap();

        (parent, index)
    }

    /// Focus the nearest sibling pane of the pane at the given index in the split.
    fn focus_nearest_sibling(&mut self, split: SplitKey, state: &mut State, index: usize) {
        // Only ever happens when closing the last pane in the root split
        // (the default widget is now displayed)
        if self.splits[split].children.len() == 0 {
            self.focused = None;
            return;
        }

        let index = index.min(self.splits[split].children.len() - 1);
        match self.splits[split].children[index] {
            Child::Pane(pane) => self.focused = Some(pane),
            Child::Split(child) => self.focus_first_pane(child, state),
        }
    }

    /// Recursively search for a pane, and focus it.
    /// Traverses only the 1st child of each encountered split.
    /// Used when a pane is closed and the sibling child in the parent split should be focused.
    fn focus_first_pane(&mut self, split: SplitKey, state: &mut State) {
        match self.splits[split].children.first() {
            Some(Child::Pane(pane)) => self.focus(*pane, state),
            Some(Child::Split(child)) => self.focus_first_pane(*child, state),
            None => self.focused = None,
        }
    }

    /// Recurse a scroll event through the splits in order to scroll the pane under the cursor.
    fn recurse_scroll(
        &mut self,
        pos: Position,
        key: SplitKey,
        event: &Event,
        state: &mut State,
    ) -> Flow {
        let split = &self.splits[key];

        if !split.area.contains(pos) {
            return Flow::NotHandled;
        }

        // Make the click relative to the split
        let rel = match split.direction {
            Direction::Horizontal => pos.x - split.area.x,
            Direction::Vertical => pos.y - split.area.y,
        };

        let mut offset = 0;
        for i in 0..split.children.len() {
            offset += split.sizes[i];

            // Check for pane collision
            if rel < offset {
                match split.children[i] {
                    Child::Pane(pane) => {
                        self.panes[pane].0.handle(event, state);
                        return Flow::Handled;
                    }
                    Child::Split(child_key) => {
                        if self.recurse_scroll(pos, child_key, event, state).handled() {
                            return Flow::Handled;
                        }
                    }
                }

                return Flow::NotHandled;
            }

            offset += 1; // border width
        }

        Flow::NotHandled
    }

    /// Recurse a click through the splits in order to:
    /// - start dragging a border
    /// - set the focused pane
    ///
    /// Events should be propagated to the focused pane if drag = None
    fn recurse_click(&self, pos: Position, key: SplitKey) -> Option<ClickResult> {
        let split = &self.splits[key];

        // Make the click relative to the split
        let rel = match split.direction {
            Direction::Horizontal => pos.x.saturating_sub(split.area.x),
            Direction::Vertical => pos.y.saturating_sub(split.area.y),
        };

        let mut offset = 0;
        for i in 0..split.children.len() {
            offset += split.sizes[i];

            // Check for pane collision
            if rel < offset {
                match split.children[i] {
                    Child::Pane(pane) => {
                        return Some(ClickResult::Pane(pane));
                    }
                    Child::Split(child_key) => {
                        if self.splits[child_key].area.contains(pos) {
                            return self.recurse_click(pos, child_key);
                        }
                    }
                }
            }

            offset += 1; // border width

            // Check for border collision
            if rel < offset {
                return Some(ClickResult::Border(key, i));
            }
        }

        None
    }

    /// Recurse focus through a click.
    /// Done in order to focus a neighboring top/bottom/left/right pane by simulating a click.
    ///
    /// Similar to recurse_click, but does not stop at borders.
    /// When a border is encountered, the pane before the border will be selected.
    fn recurse_focus(&self, pos: Position, key: SplitKey) -> Option<PaneKey> {
        let split = &self.splits[key];

        // Make the click relative to the split
        let rel = match split.direction {
            Direction::Horizontal => pos.x.saturating_sub(split.area.x),
            Direction::Vertical => pos.y.saturating_sub(split.area.y),
        };

        let mut offset = 0;
        for i in 0..split.children.len() {
            offset += split.sizes[i] + 1; // pane + next border

            // Check for pane collision
            if rel < offset {
                match split.children[i] {
                    Child::Pane(pane) => {
                        return Some(pane);
                    }
                    Child::Split(child_key) => {
                        if self.splits[child_key].area.contains(pos) {
                            return self.recurse_focus(pos, child_key);
                        }
                    }
                }
            }
        }

        None
    }

    /// Recurse through panes to focus the neighbor pane on the given side of the given pane.
    /// If there are multiple neighboring panes on the same side,
    /// the one which aligned with the cursor position is focused.
    fn focus_neighbor(&self, key: PaneKey, side: Side, cursor: Position) -> Option<PaneKey> {
        // 1. Compute click position
        let mut click = cursor;
        let (pane, _) = &self.panes[key];
        let area = pane.area();

        match side {
            // Neighbor of index i+1: need to click past the shared border to focus them.
            Side::Bottom => click.y = area.bottom() + 1,
            Side::Right => click.x = area.right() + 1,
            // Neighbor of index i-1: clicking on the shared border will focus them.
            Side::Top => {
                if area.top() == 0 {
                    return None;
                }
                click.y = area.top() - 1;
            }
            Side::Left => {
                if area.left() == 0 {
                    return None;
                }
                click.x = area.left() - 1
            }
        }

        if !self.area().contains(click) {
            return None;
        }

        self.recurse_focus(click, self.root)
    }

    fn focus(&mut self, key: PaneKey, state: &mut State) {
        self.focused = Some(key);
        self.panes[key].0.focus(state);
    }
}

// ************************************************************************* //
//                                 RENDERING                                 //
// ************************************************************************* //

impl Panes {
    fn render_child(&mut self, child: Child, area: Rect, buf: &mut Buffer, state: &State) {
        match child {
            Child::Pane(key) => self.panes[key].0.render(area, buf, state),
            Child::Split(key) => self.render_split(key, area, buf, state),
        }
    }

    fn render_split(&mut self, key: SplitKey, area: Rect, buf: &mut Buffer, state: &State) {
        self.splits[key].area = area;

        // 1. Compute interleaved areas to render the children and their borders
        let areas = Layout::new(
            self.splits[key].direction,
            once(Constraint::Fill(self.splits[key].sizes[0])).chain(
                self.splits[key].sizes.iter().skip(1).flat_map(|width| {
                    once(Constraint::Length(1)).chain(once(Constraint::Fill(*width)))
                }),
            ),
        )
        .split(area);

        // 2. Render the borders
        for i in 0..self.splits[key].children.len() - 1 {
            Border::new(self.splits[key].direction.perpendicular()).render(areas[i * 2 + 1], buf);
        }

        // 3. Render the children and update their widths for the next render
        for i in 0..self.splits[key].children.len() {
            let area = areas[i * 2];
            let child = self.splits[key].children[i];
            let width = match &self.splits[key].direction {
                Direction::Horizontal => area.width,
                Direction::Vertical => area.height,
            };
            self.splits[key].sizes[i] = width;
            self.render_child(child, area, buf, state);
        }
    }
}
