use crossterm::event::Event;
use ratatui::prelude::*;

mod border;
mod filetree;
mod home;

pub use border::Border;
pub use filetree::Filetree;
pub use home::Home;

use crate::state::State;

/// A long-lived widget that modifies a state and renders based on it.
/// The widget itself only stores UI-related states (such as Rects, scroll offsets, etc).
/// The actual data must be stored in the State type.
pub trait TedWidget {
    /// Render the widget based on the state.
    /// The widget is mutable in order to update UI-related states
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State);

    /// Handle a crossterm event and update the state accordingly.
    fn handle(&mut self, event: &Event, state: &mut State) -> bool;

    /// Returns an absolute cursor position to render.
    /// Called recursively on focused children.
    fn cursor(&self, _: &State) -> Position {
        Position::default()
    }

    /// On widget focus
    fn focus(&mut self, _: &mut State) {}

    /// Returns the area where the widget was rendered last time
    fn area(&self) -> Rect;

    fn boxed(self) -> Box<dyn TedWidget>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

/// A widget that can be cloned in a type-erased way
pub trait ClonableWidget: TedWidget {
    /// Clone a widget. Needed for splitting panes.
    fn clone(&self) -> Box<dyn ClonableWidget>;

    /// Close a widget (when its pane is removed).
    fn close(&self) {}
}
