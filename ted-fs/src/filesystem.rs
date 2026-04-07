use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use tokio::sync::mpsc::{Receiver, Sender};

use slotmap::SlotMap;

use crate::{FSEvent, File, FileKey, Folder, FolderKey, Item};

pub struct Filesystem {
    /// File tree cursor
    /// Stored here such that any widget can set the selected item
    /// instead of it being hidden away in the filetree widget.
    selected: usize,
    /// Currently peeked item: its parent folders are temporarily
    /// "peeked", which means that they are displayed as open in the filetree
    /// until another item is peeked.
    peeked: Option<Item>,

    /// Cached flattened view of the filetree for quick rendering and navigation.
    /// The usize is the depth (for indentation)
    /// Is recomputed on create/delete/move/rename/open/close
    view: Vec<(Item, usize)>,

    root: FolderKey,
    files: SlotMap<FileKey, File>,
    folders: SlotMap<FolderKey, Folder>,
    /// Lookup map by path for easy fs event handling
    paths: HashMap<PathBuf, Item>,
    unsaved: HashSet<FileKey>,
    /// Orphan files by parent folder.
    /// The keys are referenced here until the parent folder
    /// is opened and loaded in the `folders` slotmap.
    /// This allows caching parsed files displayed in the file picker.
    orphans: HashMap<PathBuf, Vec<FileKey>>,
    sender: Sender<FSEvent>,
}

impl Filesystem {
    /// Instantiate a new filesystem handler.
    /// Returns a receiver for fs events that must be fed back into the `handle_event` method.
    pub fn new() -> (Self, Receiver<FSEvent>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);

        let mut folders = SlotMap::with_key();
        let root = folders.insert(Folder::new(PathBuf::from("."), None));

        let fs = Self {
            selected: 0,
            peeked: None,
            view: vec![],
            root,
            files: SlotMap::with_key(),
            folders,
            paths: HashMap::new(),
            unsaved: HashSet::new(),
            orphans: HashMap::new(),

            sender: sender.clone(),
        };

        fs.load_folder(sender, root);
        (fs, receiver)
    }

    pub fn root_key(&self) -> FolderKey {
        self.root
    }

    pub fn root(&self) -> &Folder {
        &self.folders[self.root]
    }

    pub fn folder(&self, key: FolderKey) -> &Folder {
        &self.folders[key]
    }

    pub fn file(&self, key: FileKey) -> &File {
        &self.files[key]
    }

    pub fn file_parent(&self, key: FileKey) -> FolderKey {
        self.files[key].parent
    }

    pub fn folder_parent(&self, key: FolderKey) -> Option<FolderKey> {
        self.folders[key].parent
    }

    pub fn open(&mut self, key: FolderKey) {
        if self.folders[key].open {
            return;
        } else {
            self.folders[key].open = true;
            if !self.folders[key].init {
                self.load_folder(self.sender.clone(), key);
            } else {
                self.rebuild_view();
            }
        }
    }

    pub fn close(&mut self, key: FolderKey) {
        self.folders[key].open = false;
        self.rebuild_view();
    }

    /// Recursively close children, even through unopened folders.
    pub fn close_recurse(&mut self, key: FolderKey) {
        self.folders[key].open = false;

        for i in 0..self.folders[key].child_folders.len() {
            let child = self.folders[key].child_folders[i];
            self.close_recurse(child);
        }
        self.rebuild_view();
    }

    pub fn toggle(&mut self, key: FolderKey) {
        if self.folders[key].open {
            self.close(key);
        } else {
            self.open(key);
        }
        self.rebuild_view();
    }

    pub fn view(&self) -> &[(Item, usize)] {
        &self.view
    }

    // ************************************************* //
    //                      SELECTION                    //
    // ************************************************* //

    pub fn select_item<T: Into<Item>>(&mut self, item: T) {
        let item = item.into();
        if let Some(index) = self.view.iter().position(|(i, _)| *i == item) {
            self.selected = index;
        }
    }

    pub fn select_index(&mut self, index: usize) {
        self.selected = index.min(self.view.len().saturating_sub(1));
    }

    pub fn selected_item(&self) -> Option<Item> {
        if self.selected >= self.view.len() {
            None
        } else {
            Some(self.view[self.selected].0)
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    // ************************************************* //
    //                       MOVEMENT                    //
    // ************************************************* //

    pub fn up_n(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
    }

    pub fn down_n(&mut self, n: usize) {
        self.selected = self
            .selected
            .saturating_add(n)
            .min(self.view.len().saturating_sub(1));
    }

    pub fn up(&mut self) {
        self.up_n(1);
    }

    pub fn down(&mut self) {
        self.down_n(1);
    }

    pub fn peek<T: Into<Item>>(&mut self, item: T) {
        let item = item.into();
        if let Some(mut peeked) = self.peeked {
            if peeked == item {
                return;
            }

            // Unpeek parents
            while let Some(parent) = peeked.parent(self) {
                self.folders[parent].peeked = false;
                peeked = Item::Folder(parent);
            }
        }

        self.peeked = Some(item);

        // Peek parents
        match item.parent(self) {
            Some(mut parent) => loop {
                self.folders[parent].peeked = true;
                parent = match self.folders[parent].parent {
                    Some(p) => p,
                    None => break,
                };
            },
            None => {
                // TODO: this is an orphan item. Trigger the loading of its parents.
            }
        }

        self.rebuild_view();
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
                            folders.push(Folder::new(path, Some(key)));
                        } else {
                            files.push(File::new(path, key));
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

        self.folders[key].child_files = self.insert_files(files, key);
        self.folders[key].child_folders = self.insert_folders(folders);
        self.folders[key].init = true;
        // Insert child folders in the path lookup
        self.paths
            .extend(self.folders[key].child_folders.iter().map(|child| {
                (
                    self.folders[*child].path.clone().into(),
                    Item::Folder(*child),
                )
            }));

        self.rebuild_view();
    }

    fn rebuild_view(&mut self) {
        self.view.clear();
        recurse_view(&mut self.view, &self.files, &self.folders, self.root, 0);
        self.selected = self.selected.min(self.view.len().saturating_sub(1));
    }

    /// Insert a vec of files into the filesystem files + path lookup.
    /// Replaces incoming entries that already are in the orphans list
    /// with the existing keys, and clears the orphans for this folder.
    /// Returns the keys for injection into the parent folder struct.
    fn insert_files(&mut self, files: Vec<File>, parent: FolderKey) -> Vec<FileKey> {
        let mut keys = Vec::with_capacity(files.len());

        match self.orphans.remove(&self.folders[parent].path) {
            Some(mut orphans) => self.paths.extend(files.into_iter().map(|file| {
                let key = match orphans
                    .iter()
                    .position(|k| self.files[*k].name == file.name)
                {
                    Some(i) => orphans.swap_remove(i),
                    None => self.files.insert(file),
                };
                keys.push(key);
                (self.files[key].path.clone(), Item::File(key))
            })),
            None => self.paths.extend(files.into_iter().map(|file| {
                let key = self.files.insert(file);
                keys.push(key);
                (self.files[key].path.clone(), Item::File(key))
            })),
        };

        keys
    }

    /// Insert a vec of folders into the filesystem folders + path lookup.
    /// Returns the keys for injection into the parent folder struct.
    fn insert_folders(&mut self, folders: Vec<Folder>) -> Vec<FolderKey> {
        let mut keys = Vec::with_capacity(folders.len());

        self.paths.extend(folders.into_iter().map(|folder| {
            let key = self.folders.insert(folder);
            keys.push(key);
            (self.folders[key].path.clone(), Item::Folder(key))
        }));

        keys
    }
}

fn recurse_view(
    view: &mut Vec<(Item, usize)>,
    files: &SlotMap<FileKey, File>,
    folders: &SlotMap<FolderKey, Folder>,
    key: FolderKey,
    depth: usize,
) {
    // Traverse folders
    for folder in &folders[key].child_folders {
        view.push(((*folder).into(), depth));
        if folders[*folder].open || folders[*folder].peeked {
            recurse_view(view, files, folders, *folder, depth + 1);
        }
    }

    // Traverse files
    for file in &folders[key].child_files {
        view.push(((*file).into(), depth));
    }
}
