use std::time::{Duration, Instant};

use crate::{state::State, widgets::TedWidget};
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::prelude::*;
use ted_fs::{File, FileKey, Filesystem, Folder, FolderKey};

enum Item {
    File(FileKey),
    Folder(FolderKey),
}

/// Filetree widget
pub struct Filetree {
    rect: Rect,
    cursor: u16,
    last_click: Instant,
    /// Rendered items.
    /// Kept in memory for event handling on the rendered menu.
    items: Vec<Item>,
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            last_click: Instant::now(),
            cursor: 0,
            items: vec![],
        }
    }
}

impl TedWidget for Filetree {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let mut lines = vec![];
        let mut remaining = area.height;

        self.items.clear(); // reset rendered item keys
        self.recurse_lines(&state.fs, state.fs.root(), &mut lines, &mut remaining, 0);

        // Adjust cursor position
        if self.cursor >= lines.len() as u16 {
            self.cursor = lines.len().saturating_sub(1) as u16;
        }

        // Reprocess the lines to set the cursor style
        let lines: Vec<_> = lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                if i as u16 == self.cursor {
                    line.on_dark_gray()
                } else {
                    line
                }
            })
            .collect();

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
                        let now = Instant::now();

                        // Double click detected, toggle if it's a folder
                        if self.cursor == index
                            && now.duration_since(self.last_click) < Duration::from_millis(500)
                            && let Some(item) = self.items.get(index as usize)
                            && let Item::Folder(key) = item
                        {
                            state.fs.toggle(*key);
                        }

                        self.cursor = index;
                        self.last_click = now;
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }

        true
    }
}

impl Filetree {
    /// Move the cursor up in the file tree
    fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    /// Move the cursor down in the file tree
    fn down(&mut self) {
        self.cursor = self.cursor.saturating_add(1);
    }
    /// Toggle the open state of the selected folder
    fn toggle(&mut self, fs: &mut Filesystem) {
        if let Some(Item::Folder(key)) = self.items.get(self.cursor as usize) {
            fs.toggle(*key);
        }
    }
    /// Recursively close all children of the selected folder
    fn close(&mut self, fs: &mut Filesystem) {
        if let Some(item) = self.items.get(self.cursor as usize) {
            let key = match item {
                Item::Folder(key) => {
                    if fs.folder(*key).open {
                        *key
                    } else {
                        match fs.folder_parent(*key) {
                            Some(parent) => {
                                self.cursor = self.folder_position(parent).unwrap_or(0);
                                parent
                            }
                            None => {
                                self.cursor = 0;
                                fs.root_key()
                            }
                        }
                    }
                }
                Item::File(key) => {
                    let parent = fs.file_parent(*key);
                    self.cursor = self.folder_position(parent).unwrap_or(0);
                    parent
                }
            };
            fs.close_recurse(key);
        }
    }

    /// Returns the cursor position of the given folder in the rendered items
    fn folder_position(&self, key: FolderKey) -> Option<u16> {
        self.items
            .iter()
            .position(|item| matches!(item, Item::Folder(k) if *k == key))
            .map(|pos| pos as u16)
    }

    /// Recursively display files, folders and their children
    fn recurse_lines<'a>(
        &mut self,
        fs: &'a Filesystem,
        folder: &Folder,
        lines: &mut Vec<Line<'a>>,
        remaining: &mut u16,
        depth: usize,
    ) {
        for folder_key in &folder.child_folders {
            if *remaining == 0 {
                return;
            }

            let folder = &fs.folder(*folder_key);
            if folder.hidden() {
                continue;
            }
            lines.push(folder_line(folder, depth));
            self.items.push(Item::Folder(*folder_key)); // track rendered folder key
            *remaining -= 1;

            if folder.open {
                self.recurse_lines(fs, folder, lines, remaining, depth + 1);
            }
        }

        for file_key in &folder.child_files {
            if *remaining == 0 {
                return;
            }
            let file = &fs.file(*file_key);
            lines.push(file_line(file, depth));
            self.items.push(Item::File(*file_key)); // track rendered file key
            *remaining -= 1;
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
