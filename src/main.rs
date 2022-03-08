use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::fs::{self, metadata, DirEntry};
use std::io;
use std::path::Path;
use std::time::Duration;
use std::{env, thread};
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use tui::symbols::line::THICK_CROSS;
use tui::widgets::{List, ListItem, ListState};
use tui::Terminal;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame,
};

#[derive(Parser)]
#[clap(name = "Rusty Ranger")]
#[clap(author = "Morris Alromhein")]
#[clap(version = "1.0")]
#[clap(about = "Ranger style file explorer written in Rust")]
struct Args {
    #[clap(short = 'f', long, default_value_t = String::from("~/"))]
    filename: String,

    #[clap(short = 's', long)]
    show_hidden: bool,
}

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                // Wrap to beginning of list
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                // Wrap to end of list
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

struct App<'a> {
    current_dir: StatefulList<ListItem<'a>>,
    previous_dir: StatefulList<ListItem<'a>>,
    next_dir: StatefulList<ListItem<'a>>,

    /// Toggle to show hidden files or not
    show_hidden: bool,

    /// Files and Dirs in the current dir.
    current_dir_files: Vec<String>,
    selected_item: String,
    pwd: String,
}

impl<'a> Default for App<'a> {
    fn default() -> App<'a> {
        App {
            current_dir: StatefulList::with_items(Vec::new()),
            previous_dir: StatefulList::with_items(Vec::new()),
            next_dir: StatefulList::with_items(Vec::new()),
            show_hidden: false,
            current_dir_files: Vec::new(),
            selected_item: String::new(),
            pwd: String::new(),
        }
    }
}

fn get_file_list(path: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                let mut str = get_file_name(&entry);
                if metadata.is_dir() {
                    str.push('/');
                    result.push(str);
                }
            }
        }
    }
    result
}

fn get_file_name(entry: &DirEntry) -> String {
    entry
        .file_name()
        .into_string()
        .unwrap_or_else(|_| "Bad Dir".to_string())
}

fn main() {
    println!("Hello, world!");
}
