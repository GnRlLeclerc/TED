use crate::{state::State, widgets::TedWidget};
use crossterm::event::{Event, KeyCode};
use ratatui::prelude::*;
use ted_fs::{File, Filesystem, Folder};

/// Filetree widget
pub struct Filetree {
    rect: Rect,
    cursor: usize,
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            cursor: 0,
        }
    }
}

impl TedWidget for Filetree {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let mut lines = vec![];
        let mut remaining = area.height;
        recurse_lines(&state.fs, state.fs.root(), &mut lines, &mut remaining, 0);

        if lines.is_empty() {
            return;
        }

        if self.cursor >= lines.len() {
            self.cursor = lines.len() - 1;
        }

        apply_consume(&mut lines, self.cursor, |line| line.on_dark_gray());

        Text::from(lines).render(area, buf);
        self.rect = area;
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
                KeyCode::Down => self.cursor = self.cursor.saturating_add(1),
                _ => return false,
            },
            _ => return false,
        }

        true
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

/// Recursively display files, folders and their children
fn recurse_lines<'a>(
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

        if folder.open {
            recurse_lines(fs, folder, lines, remaining, depth + 1);
        }

        *remaining = remaining.saturating_sub(1);
    }

    for file_key in &folder.child_files {
        if *remaining == 0 {
            return;
        }

        let file = &fs.file(*file_key);
        lines.push(file_line(file, depth));
        *remaining = remaining.saturating_sub(1);
    }
}

/// Modify an item in a slice that requires consuming it to produce a new value.
fn apply_consume<T: Default>(slice: &mut [T], index: usize, f: impl Fn(T) -> T) {
    if index < slice.len() {
        let item = std::mem::take(&mut slice[index]);
        slice[index] = f(item);
    }
}
