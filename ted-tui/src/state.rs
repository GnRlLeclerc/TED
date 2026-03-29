use std::cell::Cell;

use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;

use crate::layouts::ScreenState;

/// Singleton widget states for the main app
/// widgets and layouts to be manipulated by other widgets.
pub struct WidgetStates {
    pub screens: ScreenState,
}

impl WidgetStates {
    pub fn new() -> Self {
        Self {
            screens: ScreenState::new(),
        }
    }
}

/// Global app state
pub struct State {
    pub exit: bool,
    pub fs: Filesystem,
    pub config: Config,
    pub widgets: WidgetStates,
    /// Absolute cursor position, rendered each frame
    pub cursor: Cell<Position>,
}

impl State {
    pub fn new(fs: Filesystem, config: Config) -> Self {
        Self {
            exit: false,
            fs,
            config,
            widgets: WidgetStates::new(),
            cursor: Cell::new(Position::default()),
        }
    }
}
