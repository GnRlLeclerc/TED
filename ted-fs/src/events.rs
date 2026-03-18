use crate::{FolderKey, file::File, folder::Folder};

/// Background task events used to sync the filesystem state.
pub enum FSEvent {
    FolderLoaded {
        key: FolderKey,
        files: Vec<File>,
        folders: Vec<Folder>,
    },
}
