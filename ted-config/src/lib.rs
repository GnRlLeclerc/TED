mod config;
mod watch;

pub use config::Config;
use notify_debouncer_full::{Debouncer, RecommendedCache, notify::RecommendedWatcher};

pub type ConfigWatcher = Debouncer<RecommendedWatcher, RecommendedCache>;
