use std::{fs, time::Duration};

use notify_debouncer_full::{DebounceEventResult, new_debouncer, notify::*};
use tokio::sync::mpsc::Sender;

use crate::{Config, ConfigWatcher};

pub fn watch(sender: Sender<Config>) -> (Config, ConfigWatcher) {
    let config_dir = dirs_next::config_dir().unwrap().join("ted");
    let config_path = config_dir.join("config.toml");
    // Read initial config in a blocking way
    let config =
        toml::from_str(&fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or_default();

    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        None,
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                events
                    .iter()
                    .map(|event| {
                        // Ignore non-modifying events
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {}
                            _ => return false,
                        };

                        // Toggle if any modified file is config.toml
                        event.paths.iter().any(|path| {
                            path.file_name().map_or(false, |name| name == "config.toml")
                        })
                    })
                    .any(|changed| changed)
                    .then(|| {
                        let string = fs::read_to_string(&config_path).unwrap_or_default();
                        match toml::from_str(&string) {
                            Ok(config) => {
                                if let Err(e) = sender.try_send(config) {
                                    log::error!("Config send error: {:?}", e);
                                } else {
                                    log::info!("Config updated");
                                }
                            }
                            Err(e) => log::error!("Config parse error: {:?}", e),
                        }
                    });
            }
            Err(e) => log::error!("Watch error: {:?}", e),
        },
    )
    .unwrap();

    fs::create_dir_all(&config_dir).unwrap();

    debouncer
        .watch(&config_dir, RecursiveMode::NonRecursive)
        .unwrap();

    (config, debouncer)
}
