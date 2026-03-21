use ratatui::prelude::*;
use ted_config::Config;

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
