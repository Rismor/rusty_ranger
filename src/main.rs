#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
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

struct App<'a> {
    /// Basic items.
    show_hidden: bool,
    hovered_item: String,
    hovered_index: i32,

    /// Files in the Dirs
    current_dir_vec: Vec<String<'a>>,
    previous_dir_vec: Vec<String>,
    next_dir_vec: Vec<String>,

    /// Using PathBuff object to grab files
    pwd: std::path::PathBuf,
}

impl Default for App<'a> {
    fn default() -> App {
        App {
            show_hidden: false,
            hovered_item: String::new(),
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
                    if metadata.is_dir() {
                        file_name.push('/');
                        result.push(file_name);
                    } else {
                        result.push(file_name)
                    }
                }
            }
        }
        self.hovered_item = result[0];
        result
    }

    fn previous(&mut self) {
        let i = self.hovered_index;
        i = (i - 1).rem_euclid(self.current_dir_vec.len() as i32);
        self.hovered_item.pop();
        self.hovered_item.pop();
        self.hovered_item.pop();
        self.get_selected();
        self.hovered_item.push(' ');
        self.hovered_item.push('-');
        self.hovered_item.push('>');
    }

    fn next(&mut self) {
        let i = self.hovered_index;
        i = (i + 1).rem_euclid(self.current_dir_vec.len() as i32);
        self.hovered_item.pop();
        self.hovered_item.pop();
        self.hovered_item.pop();
        self.get_selected();
        self.hovered_item.push(' ');
        self.hovered_item.push('-');
        self.hovered_item.push('>');
    }

    fn get_selected(&mut self) {
        self.hovered_item = self.current_dir_vec[self.hovered_index as usize]
    }
}

fn main() -> Result<(), io::Error> {
    let mut app = App::default();
    let args = Args::parse();

    app.pwd.push(args.filename);
    app.show_hidden = args.show_hidden;

    let child_dir = Path::new(&app.current_dir_vec[0]);
    let parent_dir = match app.pwd.parent() {
        Some(p) => p,
        None => Path::new("/"),
    };

    app.current_dir_vec = app.get_files_as_vec(&app.pwd);
    if app.pwd.ends_with("/") {
        app.next_dir_vec = app.get_files_as_vec(child_dir);
    } else {
        app.next_dir_vec = vec!["".to_string()]
    }
    if parent_dir.has_root() {
        app.previous_dir_vec = app.get_files_as_vec(parent_dir);
    } else {
        app.previous_dir_vec = vec!["".to_string()]
    }

    app.current_dir_vec.sort();
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

    // let item = app.previous_dir.items.to_owned();
    // let prev_block = List::new(item)
    //     .style(Style::default().fg(Color::White))
    //     .block(Block::default().borders(Borders::ALL));
    // f.render_widget(prev_block, chunks[0]);

    //FIXME WE cannot pass this vector into TUI LIST because of its lifetime is not good.
    let main_block = List::new(&app.current_dir_vec)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");

    f.render_stateful_widget(main_block, chunks[1], &mut app.current_dir.state);
    // f.render_widget(blockz, chunks[1]);
    let next_block = Block::default().borders(Borders::ALL);

    f.render_widget(next_block, chunks[2]);
}

//FIXME currently not able to get previous dir and next dir
// fn init_current_dir(app: &mut App) {
//     app.current_dir = StatefulList::with_items(Vec::new());

//     for file_name in app.current_dir_vec_list.clone() {
//         let file_is_hidden = match file_name.chars().next() {
//             Some('.') => true,
//             Some(_) => false,
//             None => false,
//         };

//         if !file_is_hidden || app.show_hidden {
//             app.current_dir.items.push(ListItem::new(file_name));
//         }
//     }
// app.previous_dir = StatefulList::with_items(Vec::new());
// for file_name in app.previous_dir_vec_list.clone() {
//     let file_is_hidden = match file_name.chars().next() {
//         Some('.') => true,
//         Some(_) => false,
//         None => false,
//     };

//     if !file_is_hidden || app.show_hidden {
//         app.previous_dir.items.push(ListItem::new(file_name));
//     }
// }
// }

//
//
//
fn get_files_as_vec(pwd: &Path) -> Vec<String> {
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
                if metadata.is_dir() {
                    file_name.push('/');
                    result.push(file_name);
                } else {
                    result.push(file_name)
                }
            }
        }
    }
    result
}

// TODO Allow user to go back a directory
// TODO Add showing next dir
// TODO User specified dir to open? Open that dir : open home dir
// TODO System Calls... delete copy paste rename ..
// TODO Split this file into multiple files
