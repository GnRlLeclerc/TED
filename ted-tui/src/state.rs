use ted_fs::Filesystem;

/// Global app state
pub struct State {
    pub exit: bool,
    pub fs: Filesystem,
}

impl State {
    pub fn new(fs: Filesystem) -> Self {
        Self { exit: false, fs }
    }
}
