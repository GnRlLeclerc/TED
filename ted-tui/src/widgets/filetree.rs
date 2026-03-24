use std::time::Instant;

use crate::{
    state::State,
    utils::{check_generation, scroll_to_cursor},
    widgets::TedWidget,
};
use crossterm::event::{Event, KeyCode, MouseEventKind};
use ratatui::prelude::*;
use ted_config::Config;
use ted_fs::Filesystem;

mod item;
mod recurse;

use item::Item;
use recurse::{collect_items, count_items, folder_index};

/// Filetree widget
pub struct Filetree {
    area: Rect,
    cursor: usize,
    scroll: usize,
    last_click: Instant,
    /// Selected item at the cursor position,
    /// computed from self.cursor at rendering time.
    selected: Option<Item>,
}

impl Filetree {
    pub fn new() -> Self {
        Self {
            area: Rect::default(),
            cursor: 0,
            scroll: 0,
            last_click: Instant::now(),
            selected: None,
        }
    }
}

impl TedWidget for Filetree {
    /// 1. Recursively traverse all open folders and their children
    ///    to count the maximum number of lines to render.
    /// 2. Update the cursor position to one within the amount of lines to render.
    /// 3. Update the scroll offset with respect to the cursor position, area height,
    ///    and config scroll margin.
    /// 4. Recursively traverse all open folders and their children again,
    ///    this time collecting both ratatui lines to render, and a vec of item IDs
    ///    for event handling, but only for the visible lines.
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        // 1. Count total items
        let n_items = count_items(&state.fs, state.fs.root(), &state.config);

        // 2. Update cursor position
        self.cursor = self.cursor.min(n_items.saturating_sub(1));

        // 3. Update scroll offset
        scroll_to_cursor(&mut self.scroll, self.cursor, area, &state.config);

        // 4. Recursively collect items to display
        let mut items = vec![];
        let mut count = 0;
        collect_items(
            &state.fs,
            state.fs.root(),
            &state.config,
            &mut items,
            &mut count,
            0,
            self.scroll as usize,
            area.height as usize,
        );

        // Produce lines to render + store the item at the cursor position for event handling
        Text::from(
            items
                .into_iter()
                .enumerate()
                .map(|(i, (item, depth))| {
                    if i == self.cursor.saturating_sub(self.scroll) {
                        self.selected = Some(item);
                        item.line(&state.fs, depth).on_dark_gray()
                    } else {
                        item.line(&state.fs, depth)
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
                KeyCode::Up | KeyCode::Char('k') => self.up(),
                KeyCode::Down | KeyCode::Char('j') => self.down(),
                KeyCode::Left | KeyCode::Char('h') => self.toggle(&mut state.fs),
                KeyCode::Right | KeyCode::Char('l') => self.close(&mut state.fs, &state.config),
                _ => return false,
            },
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(_) => {
                        let index = mouse.row.saturating_sub(self.area.y) as usize + self.scroll;
                        let now = Instant::now();

                        // Double click detected, toggle if it's a folder
                        if self.cursor == index
                            && now.duration_since(self.last_click)
                                < state.config.double_click_duration
                            && let Some(item) = &self.selected
                        {
                            match item {
                                Item::Folder(key) => state.fs.toggle(*key),
                                Item::File(_) => {} // TODO: open file
                            }
                        }

                        self.cursor = index;
                        self.last_click = now;
                    }
                    MouseEventKind::ScrollUp => self.scroll_up(&state.config),
                    MouseEventKind::ScrollDown => self.scroll_down(&state.config),
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

    /// Scroll one delta up. Move the cursor to stay within the visible area with
    /// the configured margin.
    fn scroll_up(&mut self, config: &Config) {
        self.scroll = self.scroll.saturating_sub(config.scroll_delta as usize);
        self.cursor = self.cursor.min(
            (self.scroll + self.area.height as usize).saturating_sub(config.scroll_margin as usize),
        );
    }

    /// Scroll one delta down. Move the cursor to stay within the visible area with
    /// the configured margin.
    fn scroll_down(&mut self, config: &Config) {
        self.scroll = self.scroll.saturating_add(config.scroll_delta as usize);
        self.cursor = self.cursor.max(self.scroll + config.scroll_margin as usize);
    }

    /// Toggle the open state of the selected folder
    fn toggle(&mut self, fs: &mut Filesystem) {
        if let Some(Item::Folder(key)) = &self.selected {
            fs.toggle(*key);
        }
    }
    /// Recursively close all children of the selected folder
    fn close(&mut self, fs: &mut Filesystem, config: &Config) {
        if let Some(item) = &self.selected {
            let key = match item {
                Item::Folder(key) => {
                    if fs.folder(*key).open {
                        *key
                    } else {
                        match fs.folder_parent(*key) {
                            Some(parent) => {
                                self.cursor = folder_index(fs, config, parent);
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
                    self.cursor = folder_index(fs, config, parent);
                    parent
                }
            };
            fs.close_recurse(key);
        }
    }
}
