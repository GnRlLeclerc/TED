use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use enum_map::{Enum, EnumMap, enum_map};
use ratatui::{layout::Offset, prelude::*};

use crate::{
    state::State,
    widgets::{Border, TedWidget},
};

#[derive(Enum, Copy, Clone)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    #[allow(dead_code)]
    pub fn vertical(&self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }

    #[allow(dead_code)]
    pub fn horizontal(&self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }
}

/// Top, bottom, left and right drawers, around a central layout widget.
///
/// In order to make the central layout accessible in a non-type erased way,
/// it is generic instead of a Box dyn.
///
/// ┌────────────────────────────────┐
/// │              Top               │
/// ├───────┬────────────────┬───────┤
/// │       │                │       │
/// │  Left │      Main      │ Right │
/// │       │                │       │
/// ├───────┴────────────────┴───────┤
/// │             Bottom             │
/// └────────────────────────────────┘
///
pub struct Drawers<T: TedWidget> {
    area: Rect,
    /// If none, the main widget is focused
    focused: Option<Side>,
    drag: Option<Side>,
    pub main: T,
    /// (widget, open) tuples
    drawers: EnumMap<Side, Option<(Box<dyn TedWidget>, bool, u16)>>,
    /// An optional overlay
    overlay: Option<(Side, Box<dyn TedWidget>, u16)>,
}

// ************************************************************************* //
//                                 PUBLIC API                                //
// ************************************************************************* //

impl<T: TedWidget> Drawers<T> {
    pub fn new(main: T) -> Self {
        let drawers = enum_map! {
            Side::Top => None,
            Side::Bottom => None,
            Side::Left => None,
            Side::Right => None,
        };

        Self {
            area: Rect::default(),
            focused: None,
            drag: None,
            main,
            drawers,
            overlay: None,
        }
    }

    pub fn with_drawer(mut self, drawer: Side, widget: Box<dyn TedWidget>, size: u16) -> Self {
        self.drawers[drawer] = Some((widget, false, size));
        self
    }

    /// Toggle a drawer open or closed.
    pub fn toggle(&mut self, drawer: Side) {
        let opposite = self.displayed_size(drawer.opposite());

        if let Some((_, open, size)) = &mut self.drawers[drawer] {
            // Check area != 0 to not clamp to 0 on initial render,
            // when the area defaults to 0.
            if !*open && self.area.area() != 0 {
                // Clamp size to avoid collision with the opposite drawer
                *size = (*size).min(match drawer.vertical() {
                    true => self.area.height.saturating_sub(opposite),
                    false => self.area.width.saturating_sub(opposite),
                });
            }

            *open = !*open
        }
    }

    /// Open an overlay on the given side.
    pub fn overlay(&mut self, side: Side, widget: Box<dyn TedWidget>, size: u16) {
        self.overlay = Some((side, widget, size));
    }
}

// ************************************************************************* //
//                                    WIDGET                                 //
// ************************************************************************* //

impl<T: TedWidget> TedWidget for Drawers<T> {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        self.area = area;

        // ***************************************************************** //
        //                    ALONG THE VERTICAL DIRECTION                   //
        // ***************************************************************** //

        let mut constraints = vec![];

        if let Some((_, open, size)) = &self.drawers[Side::Top]
            && *open
        {
            constraints.push(Constraint::Length(*size));
            constraints.push(Constraint::Length(1)); // border
        }

        constraints.push(Constraint::Fill(1));

        if let Some((_, open, size)) = &self.drawers[Side::Bottom]
            && *open
        {
            constraints.push(Constraint::Length(1)); // border
            constraints.push(Constraint::Length(*size));
        }

        let rects = Layout::vertical(constraints).split(area);

        let mut offset = 0;
        if let Some((drawer, open, _)) = &mut self.drawers[Side::Top]
            && *open
        {
            drawer.render(rects[offset], buf, state);
            Border::horizontal().render(rects[offset + 1], buf);
            offset += 2;
        }

        if let Some((drawer, open, _)) = &mut self.drawers[Side::Bottom]
            && *open
        {
            Border::horizontal().render(rects[offset], buf);
            drawer.render(rects[offset + 1], buf, state);
        }

        // ***************************************************************** //
        //                   ALONG THE HORIZONTAL DIRECTION                  //
        // ***************************************************************** //

        let mut constraints = vec![];
        if let Some((_, open, size)) = &self.drawers[Side::Left]
            && *open
        {
            constraints.push(Constraint::Length(*size));
            constraints.push(Constraint::Length(1)); // border
        }

        constraints.push(Constraint::Fill(1));

        if let Some((_, open, size)) = &self.drawers[Side::Right]
            && *open
        {
            constraints.push(Constraint::Length(1)); // border
            constraints.push(Constraint::Length(*size));
        }

        let rects = Layout::horizontal(constraints).split(rects[offset]);

        offset = 0;

        if let Some((drawer, open, _)) = &mut self.drawers[Side::Left]
            && *open
        {
            drawer.render(rects[offset], buf, state);
            Border::vertical().render(rects[offset + 1], buf);
            offset += 2;
        }

        self.main.render(rects[offset], buf, state);

        if let Some((drawer, open, _)) = &mut self.drawers[Side::Right]
            && *open
        {
            Border::vertical().render(rects[offset + 1], buf);
            drawer.render(rects[offset + 2], buf, state);
        }

        // ***************************************************************** //
        //                               OVERLAY                             //
        // ***************************************************************** //

        if let Some((side, overlay, size)) = &mut self.overlay {
            let (direction, constraints, index) = match side {
                Side::Top => (
                    Direction::Vertical,
                    [Constraint::Length(*size), Constraint::Fill(1)],
                    0,
                ),
                Side::Bottom => (
                    Direction::Vertical,
                    [Constraint::Fill(1), Constraint::Length(*size)],
                    1,
                ),
                Side::Left => (
                    Direction::Horizontal,
                    [Constraint::Length(*size), Constraint::Fill(1)],
                    0,
                ),
                Side::Right => (
                    Direction::Horizontal,
                    [Constraint::Fill(1), Constraint::Length(*size)],
                    1,
                ),
            };

            let rects = Layout::new(direction, constraints).split(area);
            overlay.render(rects[index], buf, state);
        }
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        // ***************************************************************** //
        //                               OVERLAY                             //
        // ***************************************************************** //

        if let Some((_, overlay, _)) = &mut self.overlay {
            if overlay.handle(event, state) {
                return true;
            } else {
                self.overlay = None;
            }
        }

        // ***************************************************************** //
        //                         DRAGGING / CLICKING                       //
        // ***************************************************************** //

        match event {
            Event::Mouse(mouse) => {
                let cursor = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(_) => {
                        match self.click_target(cursor) {
                            Some((side, border)) => {
                                if border {
                                    self.drag = Some(side);
                                    // Started dragging, don't propagate to the focused pane
                                    return true;
                                } else {
                                    self.focused = Some(side);
                                }
                            }
                            None => self.focused = None,
                        }
                        self.drag = None;
                    }
                    MouseEventKind::Up(_) => {
                        self.drag = None;
                    }
                    MouseEventKind::Drag(_) => {
                        if let Some(side) = self.drag {
                            let new_size = match side {
                                Side::Top => cursor.y.saturating_sub(self.area.y).min(
                                    self.area
                                        .height
                                        .saturating_sub(self.displayed_size(Side::Bottom)),
                                ),
                                Side::Bottom => {
                                    self.area.bottom().saturating_sub(cursor.y + 1).min(
                                        self.area
                                            .height
                                            .saturating_sub(self.displayed_size(Side::Top)),
                                    )
                                }
                                Side::Left => cursor.x.saturating_sub(self.area.x).min(
                                    self.area
                                        .width
                                        .saturating_sub(self.displayed_size(Side::Right)),
                                ),
                                Side::Right => self.area.right().saturating_sub(cursor.x + 1).min(
                                    self.area
                                        .width
                                        .saturating_sub(self.displayed_size(Side::Left)),
                                ),
                            };

                            if let Some((_, open, size)) = &mut self.drawers[side]
                                && *open
                            {
                                *size = new_size;
                            } else {
                                self.drag = None;
                            }

                            return true;
                        }
                    }
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        return match self.click_target(cursor) {
                            Some((side, _)) => {
                                if let Some((drawer, _, _)) = &mut self.drawers[side] {
                                    drawer.handle(event, state)
                                } else {
                                    false
                                }
                            }
                            None => self.main.handle(event, state),
                        };
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // ***************************************************************** //
        //                          FOCUSED WIDGET                           //
        // ***************************************************************** //

        if let Some(focused) = &self.focused {
            match &mut self.drawers[*focused] {
                Some((drawer, _, _)) => {
                    if drawer.handle(event, state) {
                        return true;
                    }
                }
                None => self.focused = None,
            }
        } else {
            if self.main.handle(event, state) {
                return true;
            }
        }

        // ***************************************************************** //
        //                     MOVE FOCUS / TOGGLE DRAWERS                   //
        // ***************************************************************** //

        if let Event::Key(key) = event {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                return false;
            }
            match key.code {
                // Focus left
                KeyCode::Char('h') => match self.focused {
                    Some(Side::Right) => self.focus_main(state),
                    None => self.focus_drawer(Side::Left, state),
                    _ => return false,
                },
                // Focus right
                KeyCode::Char('l') => match self.focused {
                    Some(Side::Left) => self.focus_main(state),
                    None => self.focus_drawer(Side::Right, state),
                    _ => return false,
                },
                // Focus up
                KeyCode::Char('k') => match self.focused {
                    Some(Side::Bottom) => self.focus_horizontal(state),
                    None => self.focus_drawer(Side::Top, state),
                    _ => return false,
                },
                // Focus down
                KeyCode::Char('j') => match self.focused {
                    Some(Side::Top) => self.focus_horizontal(state),
                    None => self.focus_drawer(Side::Bottom, state),
                    _ => return false,
                },
                _ => return false,
            }
            return true;
        }

        false
    }

    fn cursor(&self, state: &State) -> Position {
        if let Some(focused) = &self.focused
            && let Some((drawer, _, _)) = &self.drawers[*focused]
        {
            drawer.cursor(state)
        } else {
            self.main.cursor(state)
        }
    }
}

// ************************************************************************* //
//                              INTERNAL HELPERS                             //
// ************************************************************************* //

impl<T: TedWidget> Drawers<T> {
    /// Returns the side of the clicked drawer,
    /// and whether the click is on the border (for dragging) or not.
    fn click_target(&self, cursor: Position) -> Option<(Side, bool)> {
        let rel = cursor - Offset::from(self.area.as_position());

        if let Some((_, open, size)) = &self.drawers[Side::Top]
            && *open
        {
            let limit = *size;
            if rel.y <= limit {
                return Some((Side::Top, rel.y == limit));
            }
        }

        if let Some((_, open, size)) = &self.drawers[Side::Bottom]
            && *open
        {
            let limit = self.area.bottom() - (*size + 1);
            if rel.y >= limit {
                return Some((Side::Bottom, rel.y == limit));
            }
        }

        if let Some((_, open, size)) = &self.drawers[Side::Left]
            && *open
        {
            let limit = *size;
            if rel.x <= limit {
                return Some((Side::Left, rel.x == limit));
            }
        }

        if let Some((_, open, size)) = &self.drawers[Side::Right]
            && *open
        {
            let limit = self.area.right() - (*size + 1);
            if rel.x >= limit {
                return Some((Side::Right, rel.x == limit));
            }
        }

        None
    }

    /// Focus the widget that collides with the cursor in the given direction.
    /// Used when moving the focus up from the bottom drawer or down from the top drawer,
    /// which can fall into either the left, center or right widget depending on the cursor
    /// position.
    fn focus_horizontal(&mut self, state: &mut State) {
        let cursor = state.cursor;
        if let Some((_, open, size)) = &self.drawers[Side::Left]
            && *open
        {
            let rel = cursor.x.saturating_sub(self.area.x);
            if rel <= *size + 1 {
                self.focus_drawer(Side::Left, state);
                return;
            }
        }

        if let Some((_, open, size)) = &self.drawers[Side::Right]
            && *open
        {
            let rel = self.area.right().saturating_sub(cursor.x);
            if rel <= *size + 1 {
                self.focus_drawer(Side::Right, state);
                return;
            }
        }

        self.focused = None;
    }

    /// Focus a drawer, only if it exists and is open.
    /// Else, do nothing.
    fn focus_drawer(&mut self, side: Side, state: &mut State) {
        if let Some((drawer, open, _)) = &mut self.drawers[side]
            && *open
        {
            self.focused = Some(side);
            drawer.focus(state);
        }
    }

    fn focus_main(&mut self, state: &mut State) {
        self.focused = None;
        self.main.focus(state);
    }

    /// Returns the displayed size of the drawer + its border on the given side
    /// Used to clamp drawer sizes to avoid collision with the opposite drawer.
    fn displayed_size(&self, side: Side) -> u16 {
        if let Some((_, open, size)) = &self.drawers[side]
            && *open
        {
            *size + 1
        } else {
            0
        }
    }
}
