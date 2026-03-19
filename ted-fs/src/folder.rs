use std::path::PathBuf;

use super::{FileKey, FolderKey};

#[derive(Debug)]
pub struct Folder {
    pub path: PathBuf,
    pub parent: Option<FolderKey>,
    pub name: String,

    pub child_files: Vec<FileKey>,
    pub child_folders: Vec<FolderKey>,

    /// Current open state in UI
    pub open: bool,
    /// Whether the folder has already been loaded once
    pub init: bool,
}

impl Folder {
    pub fn new(path: PathBuf, parent: Option<FolderKey>) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self {
            path,
            parent,
            name,
            child_files: vec![],
            child_folders: vec![],
            open: false,
            init: false,
        }
    }

    pub fn hidden(&self) -> bool {
        match &self.name as &str {
            ".git" | ".venv" | "__pycache__" => true,
            _ => false,
        }
    }
}

impl PartialEq for Folder {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for Folder {}

impl PartialOrd for Folder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Folder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
