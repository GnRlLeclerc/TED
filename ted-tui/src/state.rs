use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;

/// Global app state
pub struct State {
    pub fs: Filesystem,
    pub config: Config,
    /// Absolute cursor position, rendered each frame
    pub cursor: Position,
}

impl State {
    pub fn new(fs: Filesystem, config: Config) -> Self {
        Self {
            fs,
            config,
            cursor: Position::default(),
        }
    }
}
