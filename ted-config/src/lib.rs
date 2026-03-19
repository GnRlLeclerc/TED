mod config;
mod watch;

pub use config::Config;
use notify_debouncer_mini::{Debouncer, notify::RecommendedWatcher};

pub type ConfigWatcher = Debouncer<RecommendedWatcher>;
