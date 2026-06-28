use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use devicons::FileIcon;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, sinks::UTF8};
use ignore::{WalkBuilder, WalkState};
use nucleo::Utf32String;
use tokio::{sync::watch::Sender, time::Instant};

use super::{Matcher, Tick};

pub struct GrepMatcher {
    tx: Sender<Instant>,
    running: Arc<AtomicBool>,
    items: Arc<boxcar::Vec<(Arc<PathBuf>, usize)>>,
    /// Amount of matched items (to check whether the length changed)
    length: usize,
    /// Searched path
    path: PathBuf,
}

impl GrepMatcher {
    pub fn new(tx: Sender<Instant>) -> Self {
        Self {
            tx,
            running: Arc::new(AtomicBool::new(false)),
            items: Arc::new(boxcar::Vec::new()),
            length: 0,
            path: PathBuf::from("."),
        }
    }

    pub fn selected(&self, index: usize) -> Option<&Path> {
        self.items.get(index).map(|(path, _)| path.as_path())
    }
}

// ***************************************************** //
//                     MATCHER TRAIT                     //
// ***************************************************** //

impl Matcher for GrepMatcher {
    type Data<'a> = &'a Path;
    type View<'a> = GrepView<'a>;

    fn open<'a>(&'a mut self, data: &Path) {
        self.path = data.to_path_buf();
    }

    fn search(&mut self, filter: &str, _append: bool) {
        self.close(); // Stop the previous search

        if filter.chars().count() < 3 {
            return; // Don't search for less than 3 chars
        }

        let running = self.running.clone();
        let items = self.items.clone();
        let filter = filter.to_string();
        running.store(true, Ordering::Relaxed);

        // Run ticker task
        let ticker_running = running.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            while ticker_running.load(Ordering::Relaxed) {
                let _ = tx.send(Instant::now());
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });

        // Run matcher task
        let tx = self.tx.clone();
        let walker = WalkBuilder::new(&self.path).build_parallel();
        tokio::task::spawn_blocking(move || {
            std::thread::sleep(Duration::from_millis(300)); // Debounce
            if !running.load(Ordering::Relaxed) {
                return;
            }

            walker.run(|| {
                let items = items.clone();
                let running = running.clone();
                let matcher = RegexMatcher::new(&filter).unwrap();
                let mut searcher = Searcher::new();
                Box::new(move |result| {
                    let items = items.clone();
                    if let Ok(entry) = result
                        && entry.path().is_file()
                    {
                        let path = Arc::new(entry.path().to_path_buf());
                        let _ = searcher.search_path(
                            &matcher,
                            entry.path(),
                            UTF8(move |line_number, _line| {
                                items.push((path.clone(), line_number as usize));
                                Ok(true)
                            }),
                        );
                    }

                    if !running.load(Ordering::Relaxed) {
                        return WalkState::Quit;
                    }
                    WalkState::Continue
                })
            });

            // Send the final tick if not cancelled
            if running.swap(false, Ordering::Relaxed) {
                tx.send(Instant::now()).unwrap();
            }
        });
    }

    fn tick(&mut self) -> Tick {
        let matched = self.items.count();
        let total = matched;
        let changed = matched != self.length;
        let running = self.running.load(Ordering::Relaxed);
        self.length = matched;

        Tick {
            matched,
            total,
            changed,
            running,
        }
    }

    fn close(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.running = Arc::new(AtomicBool::new(false));
        self.items = Arc::new(boxcar::Vec::new());
        self.length = 0;
    }

    fn slice<'a>(&'a self, offset: usize, limit: usize) -> Vec<Self::View<'a>> {
        self.items
            .iter()
            .skip(offset)
            .take(limit)
            .map(|(_, item)| item.into())
            .collect()
    }
}

// ***************************************************** //
//                         VIEW                          //
// ***************************************************** //

pub struct GrepView<'a> {
    pub utf32: Utf32String,
    pub string: Cow<'a, str>,
    pub icon: FileIcon,
    pub line: usize,
}

impl<'a> From<&'a (Arc<PathBuf>, usize)> for GrepView<'a> {
    fn from(data: &'a (Arc<PathBuf>, usize)) -> Self {
        let (path, line) = data;
        let string = path.to_string_lossy();

        Self {
            utf32: Utf32String::from(string.as_ref()),
            string,
            icon: FileIcon::from(path.as_path()),
            line: *line,
        }
    }
}
