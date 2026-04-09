use std::time::Instant;

use crate::{state::State, utils::scroll_to_cursor, widgets::TedWidget};
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::{File, Filesystem, Folder, Item};

/// Filetree widget
pub struct Filetree {
    area: Rect,
    scroll: usize,
    last_click: Instant,
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            scroll: 0,
            last_click: Instant::now(),
        }
    }
}

impl TedWidget for Filetree {
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        // Update scroll offset
        scroll_to_cursor(
            &mut self.scroll,
            state.fs.selected_index(),
            area,
            &state.config,
        );

        // Get the visible items
        let max = self.scroll + area.height as usize;
        let selected = state.fs.selected_index().saturating_sub(self.scroll);
        let items = &state.fs.view()[self.scroll..max.min(state.fs.view().len())];

        // Produce lines to render
        Text::from(
            items
                .iter()
                .enumerate()
                .map(|(i, (item, depth))| {
                    if i == selected {
                        item_line(item, &state.fs, *depth).on_dark_gray()
                    } else {
                        item_line(item, &state.fs, *depth)
                    }
                })
                .collect::<Vec<_>>(),
        )
        .render(area, buf);
        self.area = area;
    }

    fn handle(&mut self, event: &Event, state: &mut State) -> bool {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => state.fs.up(),
                KeyCode::Down | KeyCode::Char('j') => state.fs.down(),
                KeyCode::Left | KeyCode::Char('h') => self.toggle(&mut state.fs),
                KeyCode::Right | KeyCode::Char('l') => self.close(&mut state.fs),
                _ => return false,
            },
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(_) => {
                        let index = mouse.row.saturating_sub(self.area.y) as usize + self.scroll;
                        let now = Instant::now();

                        // Double click detected, toggle if it's a folder
                        if state.fs.selected_index() == index
                            && now.duration_since(self.last_click)
                                < state.config.double_click_duration
                            && let Some(item) = state.fs.selected_item()
                        {
                            match item {
                                Item::Folder(key) => state.fs.toggle(key),
                                Item::File(_) => {} // TODO: open file
                            }
                        }

                        state.fs.select_index(index);
                        self.last_click = now;
                    }
                    MouseEventKind::ScrollUp => self.scroll_up(&mut state.fs, &state.config),
                    MouseEventKind::ScrollDown => self.scroll_down(&mut state.fs, &state.config),
                    _ => return false,
                }
            }
            _ => return false,
        }

        true
    }

    fn cursor(&self, state: &State) -> Position {
        let index = self.area.y + (state.fs.selected_index().saturating_sub(self.scroll) as u16);
        Position::new(self.area.x, index)
    }
}

impl Filetree {
    /// Scroll one delta up. Move the cursor to stay within the visible area with
    /// the configured margin.
    fn scroll_up(&mut self, fs: &mut Filesystem, config: &Config) {
        self.scroll = self.scroll.saturating_sub(config.scroll_delta as usize);
        fs.select_index(fs.selected_index().min(
            (self.scroll + self.area.height as usize).saturating_sub(config.scroll_margin as usize),
        ));
    }

    /// Scroll one delta down. Move the cursor to stay within the visible area with
    /// the configured margin.
    fn scroll_down(&mut self, fs: &mut Filesystem, config: &Config) {
        self.scroll = self.scroll.saturating_add(config.scroll_delta as usize);
        fs.select_index(
            fs.selected_index()
                .max(self.scroll + config.scroll_margin as usize),
        );
    }

    /// Toggle the open state of the selected folder
    fn toggle(&mut self, fs: &mut Filesystem) {
        if let Some(Item::Folder(key)) = fs.selected_item() {
            fs.toggle(key);
        }
    }
    /// Recursively close all children of the selected folder
    fn close(&mut self, fs: &mut Filesystem) {
        if let Some(item) = fs.selected_item() {
            let key = match item {
                Item::Folder(key) => {
                    if fs.folder(key).open {
                        key
                    } else {
                        match fs.folder_parent(key) {
                            Some(parent) => {
                                fs.select_item(parent);
                                parent
                            }
                            None => {
                                fs.select_index(0);
                                fs.root_key()
                            }
                        }
                    }
                }
                Item::File(key) => match fs.file_parent(key) {
                    Some(parent) => {
                        fs.select_item(parent);
                        parent
                    }
                    // This is a peeked orphan file, it has no parents
                    None => {
                        return;
                    }
                },
            };
            fs.close_recurse(key);
        }
    }
}

// ************************************************************************* //
//                                  RENDERING                                //
// ************************************************************************* //

fn item_line<'a>(item: &Item, fs: &'a Filesystem, depth: usize) -> Line<'a> {
    match item {
        Item::File(key) => file_line(fs.file(*key), depth),
        Item::Folder(key) => folder_line(fs.folder(*key), depth),
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
