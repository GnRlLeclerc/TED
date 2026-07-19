use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use ted_buffer::Buffer;
use tokio::sync::mpsc::{Receiver, Sender};

use slotmap::SlotMap;

use crate::file::load_buffer;
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
    /// Lookup map to avoid loading the same paths multiple times.
    /// Used when lazy-loading orphan files previewed in the file picker,
    /// when the same file may be queried once each frame when loading the items.
    loading: HashSet<PathBuf>,
    unsaved: HashSet<FileKey>,
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
            loading: HashSet::new(),
            unsaved: HashSet::new(),

            sender: sender,
        };

        fs.load_folder(root);
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

    pub fn file_parent(&self, key: FileKey) -> Option<FolderKey> {
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
                self.load_folder(key);
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

    /// Ensure that a file is previewable from a path.
    /// Basically, ensure that the file is loaded, and its buffer as well.
    pub fn ensure_preview(&mut self, path: &Path) -> Option<FileKey> {
        match self.paths.get(path) {
            Some(Item::File(key)) => match &self.files[*key].buffer {
                Some(_) => Some(*key),
                None => {
                    self.load_buffer(*key);
                    None
                }
            },
            Some(Item::Folder(_)) => None,
            None => {
                self.load_orphan(path.to_path_buf(), true);
                None
            }
        }
    }

    pub fn preview(&self, key: FileKey) -> Option<&Buffer> {
        self.files[key].buffer.as_ref()
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

    /// Peek an item by its path.
    /// Lazy-load it if it is not loaded yet.
    /// Used by the file picker.
    /// Contrary to peek(), this function does not trigger
    /// the lazy loading of the item's parents.
    pub fn peek_path(&mut self, path: &Path) {
        match self.paths.get(path) {
            Some(item) => self.peek(*item),
            None => self.load_orphan(path.to_path_buf(), false),
        }
    }

    /// Peek an item by its key (used by editor panes to focus filetree items)
    /// Also triggers loading the parents of the item if they are not loaded yet,
    /// for displaying in the filetree.
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
            FSEvent::OrphanLoaded(file) => self.init_orphan(file),
            FSEvent::BufferLoaded { key, buffer } => self.init_buffer(key, buffer),
        }
    }
}

// ************************************************************************* //
//                               BACKGROUND TASKS                            //
// ************************************************************************* //

impl Filesystem {
    /// Load the contents of a folder asynchronously in the background
    fn load_folder(&self, key: FolderKey) {
        let sender = self.sender.clone();
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
                            files.push(File::new(path, Some(key)));
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

    /// Load an orphan file (peeked in the file picker) in the background.
    fn load_orphan(&mut self, path: PathBuf, buffer: bool) {
        if self.loading.contains(&path) {
            return;
        }
        self.loading.insert(path.clone());

        let sender = self.sender.clone();
        tokio::spawn(async move {
            if !path.is_file() {
                log::error!("Failed to load orphan: {} is not a file", path.display());
                return;
            }

            let mut file = File::new(path, None);
            if buffer {
                file.buffer = load_buffer(&file.path).await;
            }
            if let Err(err) = sender.send(FSEvent::OrphanLoaded(file)).await {
                log::error!("Failed to send orphan loaded event: {}", err);
            }
        });
    }

    /// Load the buffer of a file in the background.
    fn load_buffer(&mut self, key: FileKey) {
        if self.loading.contains(&self.files[key].path) || self.files[key].buffer.is_some() {
            return;
        }
        self.loading.insert(self.files[key].path.clone());

        let sender = self.sender.clone();
        let path = self.files[key].path.clone();
        tokio::spawn(async move {
            if let Some(buffer) = load_buffer(&path).await {
                if let Err(err) = sender.send(FSEvent::BufferLoaded { key, buffer }).await {
                    log::error!("Failed to send buffer loaded event: {}", err);
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

        self.folders[key].child_files = self.insert_files(files);
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

    /// Initialize an orphan file that is being lazily loaded.
    fn init_orphan(&mut self, file: File) {
        let key = self.files.insert(file);
        self.paths
            .insert(self.files[key].path.clone(), Item::File(key));
        self.loading.remove(&self.files[key].path);
    }

    /// Initialize a file buffer that is being lazily loaded.
    fn init_buffer(&mut self, key: FileKey, buffer: Buffer) {
        self.files[key].buffer = Some(buffer);
        self.loading.remove(&self.files[key].path);
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
    fn insert_files(&mut self, files: Vec<File>) -> Vec<FileKey> {
        let mut keys = Vec::with_capacity(files.len());

        files.into_iter().for_each(|file| {
            let key = match self.paths.get(&file.path) {
                Some(Item::File(key)) => *key,
                None => self.files.insert(file),
                // If the incoming file has the same path as an existing folder, skip it
                _ => return,
            };
            keys.push(key)
        });

        self.paths.extend(
            keys.iter()
                .map(|key| (self.files[*key].path.clone(), Item::File(*key))),
        );

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
