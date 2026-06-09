use enum_map::Enum;
use ratatui::prelude::*;
use ted_config::Config;

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

/// Update the scroll position to ensure the cursor is visible
/// with the given scroll margin.
pub fn scroll_to_cursor(scroll: &mut usize, cursor: usize, area: Rect, config: &Config) {
    let height = area.height as usize;
    let margin = config.scroll_margin as usize;

    if *scroll + margin > cursor {
        *scroll = cursor.saturating_sub(margin);
    } else if *scroll + height <= cursor + margin {
        *scroll = cursor + margin - height + 1;
    }
}

/// Update neighbor pane sizes in order to drag their shared border to the mouse position.
pub fn drag_to_cursor(
    sizes: &mut [u16],
    border: usize,
    cursor: Position,
    direction: Direction,
    area: Rect,
) {
    // Compute relative cursor position inside the split along the direction
    let rel_cursor = match direction {
        Direction::Horizontal => cursor.x.saturating_sub(area.x),
        Direction::Vertical => cursor.y.saturating_sub(area.y),
    };

    // Remove the previous panes and borders
    let size = rel_cursor.saturating_sub(sizes[..border].iter().sum::<u16>() + border as u16);

    // Max size = sum of the 2 panes being resized
    let combined = sizes[border] + sizes[border + 1];
    let size = size.min(sizes[border] + sizes[border + 1]);

    sizes[border] = size;
    sizes[border + 1] = combined - size;
}
