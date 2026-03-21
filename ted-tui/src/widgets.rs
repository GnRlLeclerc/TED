use crossterm::event::Event;
use ratatui::prelude::*;

mod border;
mod filetree;

pub use filetree::Filetree;

use crate::state::State;

/// A long-lived widget that modifies a state and renders based on it.
/// The widget itself only stores UI-related states (such as Rects, scroll offsets, etc).
/// The actual data must be stored in the State type.
pub trait TedWidget {
    /// Render the widget based on the state.
    /// The widget is mutable in order to update UI-related states
    fn render(&mut self, area: Rect, buf: &mut Buffer, fs: &State);

    /// Handle a crossterm event and update the state accordingly.
    fn handle(&mut self, event: &Event, state: &mut State) -> bool;
}
