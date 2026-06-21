use std::{borrow::Cow, path::PathBuf, sync::Arc, thread, time::Duration};

use ignore::{WalkBuilder, WalkState};

use nucleo::{
    Config, Item, Nucleo, Utf32Str, Utf32String,
    pattern::{CaseMatching, Normalization},
};

use ropey::Rope;
use ted_fs::{FileKey, Filesystem};
use tokio::{sync::watch::Receiver, time::Instant};

pub struct Matcher {
    /// Background matcher to filter all entries
    nucleo: Nucleo<PathBuf>,
    /// Last time the matcher was ticked.
    /// Used for debouncing when a new tick is received before
    /// the timeout.
    last_tick: Instant,
    /// Debouncing duration
    debounce: Duration,
    /// Finder cursor
    /// Stored here such that from the matcher tick event handling,
    /// the filesystem file preview can be updated,
    /// instead of it being hidden away in the finder widget.
    selected: usize,
    /// Total amount of items
    total: usize,
    /// Total amount of matched items
    matched: usize,
    /// Previewed file
    previewed: Option<FileKey>,
}

impl Matcher {
    pub fn new() -> (Self, Receiver<Instant>) {
        let (tx, rx) = tokio::sync::watch::channel(Instant::now());
        let config = Config::DEFAULT.match_paths();

        (
            Self {
                nucleo: Nucleo::new(
                    config,
                    Arc::new(move || {
                        // Ignore error, the matcher might run for a long time in the background
                        let _ = tx.send(Instant::now());
                    }),
                    None,
                    1,
                ),
                last_tick: Instant::now(),
                debounce: Duration::from_millis(10),
                selected: 0,
                total: 0,
                matched: 0,
                previewed: None,
            },
            rx,
        )
    }

    /// Open the matcher, scanning all files in the current directory and subdirectories.
    pub fn open(&self) {
        let injector = self.nucleo.injector();

        thread::spawn(move || {
            let walker = WalkBuilder::new(".").build_parallel();
            walker.run(|| {
                let injector = injector.clone();
                Box::new(move |result| {
                    if let Ok(entry) = result
                        && entry.path().is_file()
                    {
                        injector.push(entry.path().into(), |path, string| {
                            string[0] = Utf32String::from(path.display().to_string());
                        });
                    }

                    WalkState::Continue
                })
            });
        });
    }

    pub fn search(&mut self, filter: &str, append: bool) {
        self.nucleo.pattern.reparse(
            0,
            filter,
            CaseMatching::Ignore,
            Normalization::Smart,
            append,
        );
        self.nucleo.tick(0);
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.nucleo.restart(true);
        self.selected = 0;
        self.previewed = None;
    }

    pub fn up(&mut self, fs: &mut Filesystem) {
        self.selected = self
            .selected
            .saturating_add(1)
            .min(self.matched.saturating_sub(1));
        self.ensure_preview(fs);
    }

    pub fn down(&mut self, fs: &mut Filesystem) {
        self.selected = self.selected.saturating_sub(1);
        self.ensure_preview(fs);
    }

    pub fn ensure_preview(&mut self, fs: &mut Filesystem) {
        let snapshot = self.nucleo.snapshot();
        self.previewed = snapshot
            .get_matched_item(self.selected as u32)
            .and_then(|item| fs.ensure_preview(&item.data));
    }

    pub fn tick(&mut self, instant: Instant, fs: &mut Filesystem) -> bool {
        let status = self.nucleo.tick(0);

        // Nothing to update
        if !status.changed {
            return false;
        }

        // Last tick
        if !status.running || instant.duration_since(self.last_tick) > self.debounce {
            self.last_tick = instant;
            self.matched = self.nucleo.snapshot().matched_item_count() as usize;
            self.total = self.nucleo.snapshot().item_count() as usize;
            self.ensure_preview(fs);
            return true;
        }

        false
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn matched(&self) -> usize {
        self.matched
    }

    pub fn preview<'a>(&self, fs: &'a Filesystem) -> Option<&'a Rope> {
        self.previewed.and_then(|key| fs.preview(key))
    }

    /// Get a slice of the matched items, along with the total and matched counts.
    pub fn slice(&self, offset: u32, limit: u32) -> Vec<ItemDisplay<'_>> {
        let snapshot = self.nucleo.snapshot();
        let matched = snapshot.matched_item_count();

        let range = offset..(offset + limit).min(matched);
        let items = snapshot
            .matched_items(range.clone())
            .map(ItemDisplay::from)
            .collect::<Vec<_>>();

        items
    }
}

pub struct ItemDisplay<'a> {
    pub utf32: Utf32Str<'a>,
    pub string: Cow<'a, str>,
}

impl<'a> From<Item<'a, PathBuf>> for ItemDisplay<'a> {
    fn from(item: Item<'a, PathBuf>) -> Self {
        Self {
            utf32: item.matcher_columns[0].slice(..),
            string: item.data.to_string_lossy(),
        }
    }
}
