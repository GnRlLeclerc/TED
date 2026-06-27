use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use devicons::FileIcon;
use ignore::{WalkBuilder, WalkState};
use nucleo::{
    Config, Item, Nucleo, Utf32Str, Utf32String,
    pattern::{CaseMatching, Normalization},
};
use tokio::{sync::watch::Sender, time::Instant};

use super::{Matcher, Tick};

/// Filename matcher
pub struct FileMatcher {
    matcher: Nucleo<PathBuf>,
    cancel: Arc<AtomicBool>,
}

impl FileMatcher {
    pub fn new(tx: Sender<Instant>) -> Self {
        let config = Config::DEFAULT.match_paths();
        Self {
            matcher: Nucleo::new(
                config,
                Arc::new(move || {
                    // Ignore error, the matcher might run for a long time in the background
                    let _ = tx.send(Instant::now());
                }),
                None,
                1,
            ),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn selected(&self, index: usize) -> Option<&Path> {
        self.matcher
            .snapshot()
            .get_matched_item(index as u32)
            .map(|item| item.data.as_path())
    }
}

// ***************************************************** //
//                     MATCHER TRAIT                     //
// ***************************************************** //

impl Matcher for FileMatcher {
    type Data<'a> = &'a Path;
    type View<'a> = FileView<'a>;

    fn open(&mut self, data: &Path) {
        let injector = self.matcher.injector();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = cancel.clone();

        let walker = WalkBuilder::new(data).build_parallel();
        tokio::task::spawn_blocking(move || {
            walker.run(|| {
                let injector = injector.clone();
                let cancel = cancel.clone();
                Box::new(move |result| {
                    if let Ok(entry) = result
                        && entry.path().is_file()
                    {
                        injector.push(entry.path().into(), |path, string| {
                            string[0] = Utf32String::from(path.display().to_string());
                        });
                    }

                    if cancel.load(Ordering::Relaxed) {
                        return WalkState::Quit;
                    }
                    WalkState::Continue
                })
            });
        });
    }

    fn search(&mut self, filter: &str, append: bool) {
        self.matcher.pattern.reparse(
            0,
            filter,
            CaseMatching::Ignore,
            Normalization::Smart,
            append,
        );
        self.matcher.tick(0);
    }

    fn close(&mut self) {
        self.matcher.restart(true);
        self.cancel.store(true, Ordering::Relaxed);
    }

    fn tick(&mut self) -> Tick {
        let status = self.matcher.tick(0);
        let snapshot = self.matcher.snapshot();

        Tick {
            changed: status.changed,
            running: status.running,
            matched: snapshot.matched_item_count() as usize,
            total: snapshot.item_count() as usize,
        }
    }

    fn slice<'a>(&'a self, offset: u32, limit: u32) -> Vec<Self::View<'a>> {
        let snapshot = self.matcher.snapshot();
        let matched = snapshot.matched_item_count();

        let range = offset..(offset + limit).min(matched);
        let items = snapshot
            .matched_items(range.clone())
            .map(FileView::from)
            .collect::<Vec<_>>();

        items
    }
}

// ***************************************************** //
//                         VIEW                          //
// ***************************************************** //

pub struct FileView<'a> {
    pub utf32: Utf32Str<'a>,
    pub string: Cow<'a, str>,
    pub icon: FileIcon,
}

impl<'a> From<Item<'a, PathBuf>> for FileView<'a> {
    fn from(item: Item<'a, PathBuf>) -> Self {
        Self {
            utf32: item.matcher_columns[0].slice(..),
            string: item.data.to_string_lossy(),
            icon: FileIcon::from(item.data.as_path()),
        }
    }
}
