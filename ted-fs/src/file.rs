use std::path::{Path, PathBuf};

use devicons::FileIcon;
use hex_color::HexColor;
use ted_buffer::Buffer;
use tokio::fs;

use crate::FolderKey;

pub struct Devicon {
    pub text: String,
    pub color: Option<HexColor>,
}

impl Devicon {
    fn new(path: &Path) -> Self {
        let icon = FileIcon::from(path);

        Self {
            text: format!("{} ", icon.icon),
            color: HexColor::parse(icon.color).ok(),
        }
    }
}

pub struct File {
    /// Absolute path
    pub path: PathBuf,
    /// Optional parent (is None when the file is loaded as a peeked orphan)
    pub parent: Option<FolderKey>,
    pub name: String,
    pub icon: Devicon,

    pub buffer: Option<Buffer>,
}

impl File {
    pub fn new(path: PathBuf, parent: Option<FolderKey>) -> Self {
        let icon = Devicon::new(&path);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Self {
            path,
            parent,
            name,
            icon,
            buffer: None,
        }
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for File {}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for File {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

pub async fn load_buffer(path: &Path) -> Option<Buffer> {
    let bytes = match fs::read(path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!("Failed to read file {}: {}", path.display(), err);
            return None;
        }
    };
    Some(Buffer::new(&String::from_utf8_lossy(&bytes)))
}
