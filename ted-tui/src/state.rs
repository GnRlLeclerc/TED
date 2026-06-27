use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;
use ted_matcher::Matchers;

/// Global app state
pub struct State {
    pub fs: Filesystem,
    pub config: Config,
    pub matchers: Matchers,
    /// Absolute cursor position, rendered each frame
    pub cursor: Position,
}

impl State {
    pub fn new(fs: Filesystem, config: Config, matchers: Matchers) -> Self {
        Self {
            fs,
            config,
            matchers,
            cursor: Position::default(),
        }
    }
}
