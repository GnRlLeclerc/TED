use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use tokio::sync::mpsc::{Receiver, Sender};

use slotmap::{SlotMap, new_key_type};

use crate::{events::FSEvent, file::File, folder::Folder};

mod events;
mod file;
mod folder;

new_key_type! {
    pub struct FileKey;
    pub struct FolderKey;
}

pub struct Filesystem {
    files: SlotMap<FileKey, File>,
    folders: SlotMap<FolderKey, Folder>,
    folder_paths: HashMap<PathBuf, FolderKey>,
    unsaved: HashSet<FileKey>,
    /// Peeked files by parent folder.
    /// The keys are referenced here until the parent folder
    /// is opened and loaded in the `folders` slotmap.
    /// This allows caching parsed files displayed in the file picker.
    peeked: HashMap<PathBuf, Vec<FileKey>>,
    sender: Sender<FSEvent>,
}

impl Filesystem {
    /// Instantiate a new filesystem handler.
    /// Returns a receiver for fs events that must be fed back into the `handle_event` method.
    pub fn new() -> (Self, Receiver<FSEvent>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);

        (
            Self {
                files: SlotMap::with_key(),
                folders: SlotMap::with_key(),
                folder_paths: HashMap::new(),
                unsaved: HashSet::new(),
                peeked: HashMap::new(),

                sender,
            },
            receiver,
        )
    }

    pub fn open(&mut self, key: FolderKey) {
        if self.folders[key].open {
            return;
        } else if self.folders[key].init {
            self.folders[key].open = true;
        } else {
            self.load_folder(self.sender.clone(), key);
        }
    }

    pub fn close(&mut self, key: FolderKey) {
        self.folders[key].open = false;
    }

    /// Handle an event emitted by a background filesystem task
    pub fn handle_event(&mut self, event: FSEvent) {
        match event {
            FSEvent::FolderLoaded {
                key,
                files,
                folders,
            } => self.init_folder(key, files, folders),
        }
    }
}

// ************************************************************************* //
//                               BACKGROUND TASKS                            //
// ************************************************************************* //

impl Filesystem {
    /// Load the contents of a folder asynchronously in the background
    fn load_folder(&self, sender: Sender<FSEvent>, key: FolderKey) {
        let path = self.folders[key].path.clone();
        tokio::spawn(async move {
            let mut files: Vec<File> = vec![];
            let mut folders: Vec<Folder> = vec![];

            match tokio::fs::read_dir(&path).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_dir() {
                            folders.push(Folder::new(path));
                        } else {
                            files.push(File::new(path));
                        }
                    }

                    files.sort();
                    folders.sort();

                    if let Err(err) = sender
                        .send(FSEvent::FolderLoaded {
                            key,
                            files,
                            folders,
                        })
                        .await
                    {
                        log::error!("Failed to send folder loaded event: {}", err);
                    }
                }
                Err(err) => {
                    log::error!("Failed to read directory {}: {}", path.display(), err);
                }
            }
        });
    }
}

// ************************************************************************* //
//                               INTERNAL HELPERS                            //
// ************************************************************************* //

impl Filesystem {
    /// Initialize the contents of a folder that is being opened for the first time.
    fn init_folder(&mut self, key: FolderKey, files: Vec<File>, folders: Vec<Folder>) {
        // Avoid overwriting existing children
        if self.folders[key].init {
            return;
        }
        let file_ids = files
            .into_iter()
            .map(|file| self.files.insert(file))
            .collect::<Vec<_>>();
        let folder_ids = folders
            .into_iter()
            .map(|folder| self.folders.insert(folder))
            .collect::<Vec<_>>();

        self.folders[key].child_files = file_ids;
        self.folders[key].child_folders = folder_ids;
        self.folders[key].init = true;
        self.folder_paths
            .insert(self.folders[key].path.clone(), key);
    }
}
