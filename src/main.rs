#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
#![allow(deprecated)]
// #![allow(dead_code)]
use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::fs::{self, metadata, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, thread};
use tui::backend::CrosstermBackend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::symbols::line::THICK_CROSS;
use tui::widgets::{List, ListItem, ListState, Sparkline};
use tui::Terminal;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame,
};

use dirs;

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
}

struct App {
    /// Basic items.
    show_hidden: bool,
    hovered_index: i32,

    /// Files in the Dirs
    current_dir_vec: Vec<String>,
    previous_dir_vec: Vec<String>,
    next_dir_vec: Vec<String>,

    /// Using PathBuff object to grab files
    pwd: std::path::PathBuf,
}

impl Default for App {
    fn default() -> App {
        App {
            show_hidden: false,
            hovered_index: 0,
            pwd: match dirs::home_dir() {
                Some(x) => x,
                None => std::path::PathBuf::new(),
            },
            current_dir_vec: Vec::new(),
            previous_dir_vec: Vec::new(),
            next_dir_vec: Vec::new(),
        }
    }
}

impl App {
    fn get_files_as_vec(&mut self, pwd: &Path) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        if let Ok(entries) = fs::read_dir(pwd) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let mut file_name = {
                        let entry = &entry;
                        entry
                            .file_name()
                            .into_string()
                            .unwrap_or_else(|_| "Bad Dir".to_string())
                    };
                    let file_is_hidden = match file_name.chars().next() {
                        Some('.') => true,
                        Some(_) => false,
                        None => false,
                    };
                    if !file_is_hidden || self.show_hidden {
                        if metadata.is_dir() {
                            file_name.push('/');
                            result.push(file_name);
                        } else {
                            result.push(file_name)
                        }
                    } else {
                        continue;
                    }
                }
            }
        }
        result
    }

    fn previous(&mut self) {
        if self.current_dir_vec.len() == 0 {
            return;
        }
        let str = &self.current_dir_vec[self.hovered_index as usize];
        let str = str.as_bytes();
        if str[str.len() - 1] as char == '>' {
            self.current_dir_vec[self.hovered_index as usize].pop();
            self.current_dir_vec[self.hovered_index as usize].pop();
            self.current_dir_vec[self.hovered_index as usize].pop();
        }
        self.hovered_index = (self.hovered_index - 1).rem_euclid(self.current_dir_vec.len() as i32);
        self.current_dir_vec[self.hovered_index as usize].push(' ');
        self.current_dir_vec[self.hovered_index as usize].push('-');
        self.current_dir_vec[self.hovered_index as usize].push('>');
    }

    fn next(&mut self) {
        if self.current_dir_vec.len() == 0 {
            return;
        }
        let str = &self.current_dir_vec[self.hovered_index as usize];
        let str = str.as_bytes();
        if str[str.len() - 1] as char == '>' {
            self.current_dir_vec[self.hovered_index as usize].pop();
            self.current_dir_vec[self.hovered_index as usize].pop();
            self.current_dir_vec[self.hovered_index as usize].pop();
        }
        self.hovered_index = (self.hovered_index + 1).rem_euclid(self.current_dir_vec.len() as i32);
        self.current_dir_vec[self.hovered_index as usize].push(' ');
        self.current_dir_vec[self.hovered_index as usize].push('-');
        self.current_dir_vec[self.hovered_index as usize].push('>');
    }

    fn into_dir(&mut self) {
        self.previous_dir_vec = Vec::new();
        let mut pp = self.pwd.clone();
        self.previous_dir_vec = self.get_files_as_vec(&pp);
        self.current_dir_vec[self.hovered_index as usize].pop();
        self.current_dir_vec[self.hovered_index as usize].pop();
        self.current_dir_vec[self.hovered_index as usize].pop();
        pp.push(self.current_dir_vec[self.hovered_index as usize].clone());
        self.current_dir_vec = Vec::new();
        self.current_dir_vec = self.get_files_as_vec(&pp);
        self.hovered_index = 0;
        self.next_dir_vec = vec![
            self.pwd.to_str().unwrap().to_string(),
            pp.to_str().unwrap().to_string(),
        ]
    }
}

fn main() -> Result<(), io::Error> {
    let mut app = App::default();
    let args = Args::parse();

    app.pwd.push(args.filename);
    app.show_hidden = args.show_hidden;

    // let child_dir = Path::new(&app.current_dir_vec[0]);

    //     Some(p) => p,
    //     None => Path::new("/"),
    // };
    let pwd = app.pwd.clone();

    app.current_dir_vec = app.get_files_as_vec(&pwd);
    app.current_dir_vec.sort();

    let mut parent_dir = PathBuf::new();
    parent_dir.push(match app.pwd.parent() {
        Some(p) => p,
        None => Path::new("/"),
    });

    let mut child_dir = PathBuf::new();
    let mut pp = app.pwd.clone();
    pp.push(app.current_dir_vec[0].clone());
    child_dir.push(pp);
    if app.pwd.metadata().unwrap().is_dir() {
        app.next_dir_vec = app.get_files_as_vec(&child_dir);
    } else {
        app.next_dir_vec = vec!["pppp".to_string()]
    }
    if parent_dir.has_root() {
        app.previous_dir_vec = app.get_files_as_vec(&parent_dir);
    } else {
        app.previous_dir_vec = vec!["".to_string()]
    }

    app.next_dir_vec.sort();
    app.previous_dir_vec.sort();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    //     app.selected_item = thing.to_string();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Down => app.next(),
                KeyCode::Char('j') => app.next(),
                KeyCode::Up => app.previous(),
                KeyCode::Char('k') => app.previous(),
                KeyCode::Char('l') => app.into_dir(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        // .margin(0)
        .vertical_margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(33),
                Constraint::Percentage(45),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Percentage(3),
                Constraint::Percentage(33),
                Constraint::Percentage(45),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title_block = Block::default()
        .title(app.pwd.display().to_string())
        // .borders(Borders::BOTTOM)
        .title_style(Style::default().fg(Color::Cyan));

    f.render_widget(title_block, title_layout[0]);

    let mut items: Vec<ListItem> = Vec::new();
    for file_name in app.previous_dir_vec.clone() {
        let file_is_hidden = match file_name.chars().next() {
            Some('.') => true,
            Some(_) => false,
            None => false,
        };
        if !file_is_hidden || app.show_hidden {
            items.push(ListItem::new(file_name))
        }
    }
    let prev_block = List::new(items)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(prev_block, chunks[0]);

    let mut items: Vec<ListItem> = Vec::new();
    for file_name in app.current_dir_vec.clone() {
        let file_is_hidden = match file_name.chars().next() {
            Some('.') => true,
            Some(_) => false,
            None => false,
        };
        if !file_is_hidden || app.show_hidden {
            items.push(ListItem::new(file_name))
        }
    }
    let main_block = List::new(items)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(main_block, chunks[1]);

    let mut items: Vec<ListItem> = Vec::new();
    for file_name in app.next_dir_vec.clone() {
        let file_is_hidden = match file_name.chars().next() {
            Some('.') => true,
            Some(_) => false,
            None => false,
        };
        if !file_is_hidden || app.show_hidden {
            items.push(ListItem::new(file_name))
        }
    }
    let next_block = List::new(items)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(next_block, chunks[2]);
}

// TODO Allow user to go back a directory
// TODO Add showing next dir
// TODO User specified dir to open? Open that dir : open home dir
// TODO System Calls... delete copy paste rename ..
// TODO Split this file into multiple files
// TODO only show shown items in current_dir
//  Create new vec?
//Handle scrolling first
