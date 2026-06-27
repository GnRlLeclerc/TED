use std::time::Duration;

use ropey::Rope;
use ted_fs::{FileKey, Filesystem};
use tokio::{sync::watch::Receiver, time::Instant};

use crate::{
    matchers::{Matcher, Tick, file::FileMatcher},
    modes::MatcherMode,
};

mod matchers;
mod modes;

pub mod views {
    pub use crate::matchers::file::FileView;
}

pub use modes::{MatcherData, MatcherView};

pub struct Matchers {
    mode: MatcherMode,
    files: FileMatcher,

    /// Selected entry index
    selected: usize,

    /// Last tick result
    tick: Tick,

    /// Previewed file
    previewed: Option<FileKey>,

    /// Tick debouncing delay
    debouncing: Duration,
}

impl Matchers {
    pub fn new() -> (Self, Receiver<Instant>) {
        let (tx, rx) = tokio::sync::watch::channel(Instant::now());

        (
            Self {
                mode: MatcherMode::File,
                files: FileMatcher::new(tx),
                selected: 0,
                tick: Tick::default(),
                previewed: None,
                debouncing: Duration::from_millis(10),
            },
            rx,
        )
    }

    pub fn open(&mut self, data: MatcherData) {
        self.mode = (&data).into();
        self.selected = 0;

        match data {
            MatcherData::File(path) => self.files.open(path),
            MatcherData::Grep(_path) => todo!("open grep matcher"),
        }
    }

    pub fn search(&mut self, filter: &str, append: bool) {
        match self.mode {
            MatcherMode::File => self.files.search(filter, append),
            MatcherMode::Grep => todo!("search grep matcher"),
        }
        self.selected = 0;
    }

    pub fn close(&mut self) {
        match self.mode {
            MatcherMode::File => self.files.close(),
            MatcherMode::Grep => todo!("close grep matcher"),
        }
        self.selected = 0;
        self.previewed = None;
    }

    pub fn up(&mut self) {
        self.selected = self
            .selected
            .saturating_add(1)
            .min(self.tick.matched.saturating_sub(1));
    }

    pub fn down(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn total(&self) -> usize {
        self.tick.total
    }

    pub fn matched(&self) -> usize {
        self.tick.matched
    }

    /// Tick the correct matcher and returns true if something has changed.
    pub fn tick(&mut self, instant: Instant) -> bool {
        let tick = match self.mode {
            MatcherMode::File => self.files.tick(),
            MatcherMode::Grep => todo!("tick grep matcher"),
        };

        if !tick.changed || (tick.running && instant.elapsed() < self.debouncing) {
            return false;
        }

        self.tick = tick;
        true
    }

    pub fn slice(&self, offset: u32, limit: u32) -> MatcherView<'_> {
        match self.mode {
            MatcherMode::File => self.files.slice(offset, limit).into(),
            MatcherMode::Grep => todo!("slice grep matcher"),
        }
    }

    pub fn ensure_preview(&mut self, fs: &mut Filesystem) {
        self.previewed = match self.mode {
            MatcherMode::File => self
                .files
                .selected(self.selected)
                .and_then(|path| fs.ensure_preview(path)),
            MatcherMode::Grep => todo!("ensure preview grep matcher"),
        };
    }

    pub fn preview<'a>(&self, fs: &'a Filesystem) -> Option<&'a Rope> {
        self.previewed.as_ref().and_then(|key| fs.preview(*key))
    }
}
