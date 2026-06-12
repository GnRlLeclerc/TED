use std::ops::ControlFlow;

use crossterm::event::Event;
use ratatui::prelude::*;

mod border;
mod filetree;
mod finder;
mod home;

pub use border::Border;
pub use filetree::Filetree;
pub use finder::Finder;
pub use home::Home;

use crate::state::State;

pub enum Handled {
    /// Event handled, just stop propagating
    Handled,
    /// Event handled and widget should be closed
    Close,
}

/// Event handling result.
/// Uses ControlFlow for easy propagation.
pub type Flow = ControlFlow<Handled>;

pub trait FlowExt {
    fn handled() -> Flow {
        Flow::Break(Handled::Handled)
    }
    fn close() -> Flow {
        Flow::Break(Handled::Close)
    }
    fn not_handled() -> Flow {
        Flow::Continue(())
    }

    /// Run a callback if the flow is close, handling the close event
    /// and returning Flow::handled() instead.
    fn on_close<F: FnOnce()>(self, callback: F) -> Flow;
}

impl FlowExt for Flow {
    fn on_close<F: FnOnce()>(self, callback: F) -> Flow {
        if matches!(self, Flow::Break(Handled::Close)) {
            callback();
            Flow::handled()
        } else {
            self
        }
    }
}

/// A long-lived widget that modifies a state and renders based on it.
/// The widget itself only stores UI-related states (such as Rects, scroll offsets, etc).
/// The actual data must be stored in the State type.
pub trait TedWidget {
    /// Render the widget based on the state.
    /// The widget is mutable in order to update UI-related states
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State);

    /// Handle a crossterm event and update the state accordingly.
    fn handle(&mut self, event: &Event, state: &mut State) -> Flow;

    /// Returns an absolute cursor position to render.
    /// Called recursively on focused children.
    fn cursor(&self, _: &State) -> Position {
        self.area().as_position()
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
