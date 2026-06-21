use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;
use ted_matcher::Matcher;

/// Global app state
pub struct State {
    pub fs: Filesystem,
    pub config: Config,
    pub matcher: Matcher,
    /// Absolute cursor position, rendered each frame
    pub cursor: Position,
}

impl State {
    pub fn new(fs: Filesystem, config: Config, matcher: Matcher) -> Self {
        Self {
            fs,
            config,
            matcher,
            cursor: Position::default(),
        }
    }
}
