use crate::{state::State, widgets::TedWidget};
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::prelude::*;
use ted_fs::{File, FileKey, Filesystem, Folder, FolderKey};

enum Selection {
    File(FileKey),
    Folder(FolderKey),
    None,
}

/// Filetree widget
pub struct Filetree {
    rect: Rect,
    cursor: u16,
    selection: Selection,
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            cursor: 0,
            selection: Selection::None,
        }
    }
}

impl TedWidget for Filetree {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let mut lines = vec![];
        let mut count = 0;
        let total = area.height;
        self.recurse_lines(&state.fs, state.fs.root(), &mut lines, &mut count, total, 0);

        Text::from(lines).render(area, buf);
        self.rect = area;
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => self.up(),
                KeyCode::Down | KeyCode::Char('j') => self.down(),
                KeyCode::Left | KeyCode::Char('h') => self.toggle(&mut state.fs),
                KeyCode::Right | KeyCode::Char('l') => self.close(&mut state.fs),
                _ => return false,
            },
            Event::Mouse(mouse) => {
                let index = mouse.row.saturating_sub(self.rect.y);

                match mouse.kind {
                    MouseEventKind::Down(_) => {
                        self.cursor = index;
                    }
                    _ => {}
                }
            }
            _ => return false,
        }

        true
    }
}

impl Filetree {
    fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    fn down(&mut self) {
        self.cursor = self.cursor.saturating_add(1);
    }
    fn toggle(&mut self, fs: &mut Filesystem) {
        if let Selection::Folder(key) = self.selection {
            fs.toggle(key);
        }
    }
    fn close(&mut self, fs: &mut Filesystem) {
        let key = match self.selection {
            Selection::Folder(key) => {
                if fs.folder(key).open {
                    key
                } else {
                    fs.folder_parent(key)
                }
            }
            Selection::File(key) => fs.file_parent(key),
            _ => return,
        };
        fs.close(key);
    }

    /// Recursively display files, folders and their children
    fn recurse_lines<'a>(
        &mut self,
        fs: &'a Filesystem,
        folder: &Folder,
        lines: &mut Vec<Line<'a>>,
        count: &mut u16,
        total: u16,
        depth: usize,
    ) {
        for folder_key in &folder.child_folders {
            if *count == total {
                return;
            }

            let folder = &fs.folder(*folder_key);
            if folder.hidden() {
                continue;
            }
            let mut line = folder_line(folder, depth);
            if *count == self.cursor {
                line = line.on_dark_gray();
                self.selection = Selection::Folder(*folder_key);
            }
            lines.push(line);
            *count += 1;

            if folder.open {
                self.recurse_lines(fs, folder, lines, count, total, depth + 1);
            }
        }

        for file_key in &folder.child_files {
            if *count == total {
                return;
            }
            let file = &fs.file(*file_key);
            let mut line = file_line(file, depth);
            if *count == self.cursor {
                self.selection = Selection::File(*file_key);
                line = line.on_dark_gray();
            }
            lines.push(line);
            *count += 1;
        }
    }
}

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
