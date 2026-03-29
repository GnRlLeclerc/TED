use std::cell::Cell;

use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;

/// Global app state
pub struct State {
    pub exit: bool,
    pub fs: Filesystem,
    pub config: Config,
    /// Absolute cursor position, rendered each frame
    pub cursor: Cell<Position>,
}

impl State {
    pub fn new(fs: Filesystem, config: Config) -> Self {
        Self {
            exit: false,
            fs,
            config,
            cursor: Cell::new(Position::default()),
        }
    }
}
