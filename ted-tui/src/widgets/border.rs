use ratatui::prelude::*;

pub struct Border {
    direction: Direction,
    style: Style,
}

/// A line border.
/// After rendering, checks for contact with neighboring borders
/// and updates the intersection characters accordingly.
impl Border {
    pub fn new(direction: Direction) -> Self {
        let style = Style::default().dark_gray();
        Self { direction, style }
    }

    pub fn horizontal() -> Self {
        Self::new(Direction::Horizontal)
    }
    pub fn vertical() -> Self {
        Self::new(Direction::Vertical)
    }
}

impl Widget for Border {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.direction {
            Direction::Vertical => {
                for x in area.left()..area.right() {
                    buf.set_string(x, area.top(), "─", self.style);
                }

                if area.left() > 0
                    && let Some(cell) = buf.cell_mut(Position::new(area.left() - 1, area.top()))
                {
                    match cell.symbol() {
                        "│" => {
                            cell.set_symbol("├");
                        }
                        "┤" => {
                            cell.set_symbol("┼");
                        }
                        _ => {}
                    };
                }

                if let Some(cell) = buf.cell_mut(Position::new(area.right(), area.top())) {
                    match cell.symbol() {
                        "│" => {
                            cell.set_symbol("┤");
                        }
                        "├" => {
                            cell.set_symbol("┼");
                        }
                        _ => {}
                    };
                }
            }
            Direction::Horizontal => {
                for y in area.top()..area.bottom() {
                    buf.set_string(area.left(), y, "│", self.style);
                }

                if area.top() > 0
                    && let Some(cell) = buf.cell_mut(Position::new(area.left(), area.top() - 1))
                {
                    match cell.symbol() {
                        "─" => {
                            cell.set_symbol("┬");
                        }
                        "┴" => {
                            cell.set_symbol("┼");
                        }
                        _ => {}
                    };
                }

                if let Some(cell) = buf.cell_mut(Position::new(area.left(), area.bottom())) {
                    match cell.symbol() {
                        "─" => {
                            cell.set_symbol("┴");
                        }
                        "┬" => {
                            cell.set_symbol("┼");
                        }
                        _ => {}
                    };
                }
            }
        }
    }
}
