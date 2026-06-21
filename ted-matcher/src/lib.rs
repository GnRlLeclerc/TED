use std::{borrow::Cow, path::PathBuf, sync::Arc, thread, time::Duration};

use ignore::{WalkBuilder, WalkState};

use nucleo::{
    Config, Item, Nucleo, Utf32Str, Utf32String,
    pattern::{CaseMatching, Normalization},
};

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
        self.tick(Instant::now());
    }

    pub fn close(&mut self) {
        self.nucleo.restart(true);
    }

    pub fn tick(&mut self, instant: Instant) -> bool {
        let status = self.nucleo.tick(0);

        // Nothing to update
        if !status.changed {
            return false;
        }

        // Last tick
        if !status.running || instant.duration_since(self.last_tick) > self.debounce {
            self.last_tick = instant;
            return true;
        }

        false
    }

    /// Get a slice of the matched items, along with the total and matched counts.
    pub fn slice(&self, offset: u32, limit: u32) -> (Vec<ItemDisplay<'_>>, u32, u32) {
        let snapshot = self.nucleo.snapshot();
        let total = snapshot.item_count();
        let matched = snapshot.matched_item_count();

        let range = offset..(offset + limit).min(matched);
        let items = snapshot
            .matched_items(range.clone())
            .map(ItemDisplay::from)
            .collect::<Vec<_>>();

        (items, total, matched)
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
