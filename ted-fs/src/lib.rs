mod events;
mod file;
mod filesystem;
mod folder;

pub use events::FSEvent;
pub use file::File;
pub use filesystem::Filesystem;
pub use folder::Folder;

use slotmap::new_key_type;

new_key_type! {
    pub struct FileKey;
    pub struct FolderKey;
}
