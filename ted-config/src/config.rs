use std::time::Duration;

use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

use crate::{ConfigWatcher, watch::watch};

mod duration_millis {
    use serde::{Deserialize, Deserializer};
    use std::time::Duration;

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let millis = u64::deserialize(d)?;
        Ok(Duration::from_millis(millis))
    }
}

fn double_click_duration() -> Duration {
    Duration::from_millis(500)
}

fn scroll_margin() -> u16 {
    5
}

fn ignored_folders() -> Vec<String> {
    [".git", ".venv", "__pycache__"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "double_click_duration", with = "duration_millis")]
    pub double_click_duration: Duration,
    #[serde(default = "scroll_margin")]
    pub scroll_margin: u16,
    #[serde(default = "ignored_folders")]
    ignored_folders: Vec<String>,
}

impl Config {
    pub fn new() -> (Self, Receiver<Self>, ConfigWatcher) {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);

        (Self::default(), receiver, watch(sender))
    }

    pub fn ignored_folder(&self, name: &str) -> bool {
        self.ignored_folders.contains(&name.to_string())
    }
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}
