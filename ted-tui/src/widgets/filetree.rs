use crate::{state::State, widgets::TedWidget};
use crossterm::event::Event;
use ratatui::prelude::*;
use ted_fs::{File, Filesystem, Folder};

/// Filetree widget
pub struct Filetree {
    rect: Rect,
    // TODO: cursor offset
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
        }
    }
}

impl TedWidget for Filetree {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let mut lines = vec![];
        let mut remaining = area.height;
        recurse_lines(&state.fs, state.fs.root(), &mut lines, &mut remaining, 0);
        Text::from(lines).render(area, buf);
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        // TODO: change cursor offset with keys
        false
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
