use std::{fs, time::Duration};

use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::*};
use tokio::sync::mpsc::Sender;

use crate::{Config, ConfigWatcher};

pub fn watch(sender: Sender<Config>) -> ConfigWatcher {
    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        move |res: DebounceEventResult| match res {
            Ok(events) => events.iter().for_each(|event| {
                if let Some(name) = event.path.file_name()
                    && name == "config.toml"
                {
                    let string = fs::read_to_string(&event.path).unwrap_or_default();
                    match toml::from_str(&string) {
                        Ok(config) => {
                            if let Err(e) = sender.try_send(config) {
                                log::error!("Config send error: {:?}", e);
                            }
                        }
                        Err(e) => log::error!("Config parse error: {:?}", e),
                    }
                }
            }),
            Err(e) => log::error!("Watch error: {:?}", e),
        },
    )
    .unwrap();

    let config_dir = dirs_next::config_dir().unwrap().join("ted");

    fs::create_dir_all(&config_dir).unwrap();

    debouncer
        .watcher()
        .watch(&config_dir, RecursiveMode::NonRecursive)
        .unwrap();

    debouncer
}
