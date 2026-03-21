use ratatui::prelude::*;

use ted_fs::{File, FileKey, Filesystem, Folder, FolderKey};

/// A filetree item, either a file or a folder.
#[derive(Clone, Copy)]
pub enum Item {
    File(FileKey),
    Folder(FolderKey),
}

impl Item {
    /// Produce a line to render for this item
    pub fn line<'a>(&self, fs: &'a Filesystem, depth: usize) -> Line<'a> {
        match self {
            Item::File(key) => file_line(fs.file(*key), depth),
            Item::Folder(key) => folder_line(fs.folder(*key), depth),
        }
    }
}

impl From<&FileKey> for Item {
    fn from(key: &FileKey) -> Self {
        Self::File(*key)
    }
}

impl From<&FolderKey> for Item {
    fn from(key: &FolderKey) -> Self {
        Self::Folder(*key)
    }
}

// ***************************************************** //
//                        Rendering                      //
// ***************************************************** //

fn file_line(file: &File, depth: usize) -> Line<'_> {
    let mut style = Style::default();
    if let Some(color) = file.icon.color {
        style = style.fg(Color::Rgb(color.r, color.g, color.b));
    }

    Line::from(vec![
        Span::raw("  ".repeat(depth + 1)),
        Span::styled(&file.icon.text, style),
        Span::raw(&file.name),
    ])
}

fn folder_line(folder: &Folder, depth: usize) -> Line<'_> {
    Line::from(vec![
        Span::raw("  ".repeat(depth)),
        Span::raw(if folder.open { " " } else { " " }).gray(),
        Span::raw(if folder.open { " " } else { " " }).blue(),
        Span::raw(&folder.name).blue(),
    ])
}
