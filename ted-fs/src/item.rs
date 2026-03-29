use crate::{FileKey, Filesystem, FolderKey};

/// A filesystem item
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    File(FileKey),
    Folder(FolderKey),
}

impl Item {
    pub fn parent(&self, fs: &Filesystem) -> Option<FolderKey> {
        match self {
            Self::File(key) => Some(fs.file_parent(*key)),
            Self::Folder(key) => fs.folder_parent(*key),
        }
    }
}

impl From<FileKey> for Item {
    fn from(key: FileKey) -> Self {
        Self::File(key)
    }
}

impl From<FolderKey> for Item {
    fn from(key: FolderKey) -> Self {
        Self::Folder(key)
    }
}

impl PartialEq<FileKey> for Item {
    fn eq(&self, other: &FileKey) -> bool {
        matches!(self, Self::File(key) if key == other)
    }
}

impl PartialEq<FolderKey> for Item {
    fn eq(&self, other: &FolderKey) -> bool {
        matches!(self, Self::Folder(key) if key == other)
    }
}
