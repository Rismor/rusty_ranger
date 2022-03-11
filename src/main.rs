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

enum Context {
    GoNext,
    GoPrev,
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
    // next_dir: StatefulList<ListItem<'a>>,
    /// Toggle to show hidden files or not
    show_hidden: bool,

    /// Files and Dirs in the current dir.
    current_dir_vec_list: Vec<String>,
    previous_dir_vec_list: Vec<String>,
    next_dir_vec_list: Vec<String>,

    // Using Path Object instead of string.
    pwd: std::path::PathBuf,
}

impl<'a> Default for App<'a> {
    fn default() -> App<'a> {
        App {
            current_dir: StatefulList::with_items(Vec::new()),
            previous_dir: StatefulList::with_items(Vec::new()),
            // next_dir: StatefulList::with_items(Vec::new()),
            show_hidden: false,
            current_dir_vec_list: Vec::new(),
            pwd: match dirs::home_dir() {
                Some(x) => x,
                None => std::path::PathBuf::new(),
            },
            previous_dir_vec_list: Vec::new(),
            next_dir_vec_list: Vec::new(),
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

    // let items: Vec<ListItem> = Vec::new();
    // let items = &*app.current_dir.items;
    // let items = List::new(items)
    //     .style(Style::default().fg(Color::White))
    //     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
    //     .highlight_symbol(">> ");

    // let items = List::new(&*app.current_dir.items)
    //     .style(Style::default().fg(Color::Blue))
    //     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
    //     .highlight_symbol(">> ");

    let item = app.previous_dir.items.to_owned();
    let prev_block = List::new(item)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(prev_block, chunks[0]);

    let title_block = Block::default();
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

    let item = app.current_dir.items.to_owned();
    let main_block = List::new(item)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");

    f.render_stateful_widget(main_block, chunks[1], &mut app.current_dir.state);
    // f.render_widget(blockz, chunks[1]);
    let next_block = Block::default().borders(Borders::ALL);
    let testing_block = Block::default()
        .title(app.pwd.display().to_string())
        // .borders(Borders::BOTTOM)
        .title_style(Style::default().fg(Color::Cyan));

    f.render_widget(testing_block, title_layout[0]);

    f.render_widget(next_block, chunks[2]);
}

fn main() -> Result<(), io::Error> {
    let mut app = App::default();
    let args = Args::parse();

    // app.pwd = args.filename;
    app.pwd.push(args.filename);
    app.show_hidden = args.show_hidden;
    app.current_dir_vec_list = get_files_as_vec(&app.pwd);
    app.previous_dir_vec_list = get_files_as_vec(app.pwd.parent().unwrap());

    // app.previous_dir_files = get_file_list();

    app.current_dir_vec_list.sort();

    init_current_dir(&mut app);

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
                KeyCode::Left => app.current_dir.unselect(),
                KeyCode::Char('h') => app.current_dir.unselect(),
                KeyCode::Down => app.current_dir.next(),
                KeyCode::Char('j') => app.current_dir.next(),
                KeyCode::Up => app.current_dir.previous(),
                KeyCode::Char('k') => app.current_dir.previous(),
                _ => {}
            }
        }
    }
}

//FIXME currently not able to get previous dir and next dir
fn init_current_dir(app: &mut App) {
    app.current_dir = StatefulList::with_items(Vec::new());

    for file_name in app.current_dir_vec_list.clone() {
        let file_is_hidden = match file_name.chars().next() {
            Some('.') => true,
            Some(_) => false,
            None => false,
        };

        if !file_is_hidden || app.show_hidden {
            app.current_dir.items.push(ListItem::new(file_name));
        }
    }
    app.previous_dir = StatefulList::with_items(Vec::new());
    for file_name in app.previous_dir_vec_list.clone() {
        let file_is_hidden = match file_name.chars().next() {
            Some('.') => true,
            Some(_) => false,
            None => false,
        };

        if !file_is_hidden || app.show_hidden {
            app.previous_dir.items.push(ListItem::new(file_name));
        }
    }
}

fn update(context: Context, app: &mut App) {
    init_current_dir(app);
    match context {
        Context::GoNext => {
            // app.previous_dir.items = app.current_dir.items.to_owned();
            let i = app.current_dir.state.selected().unwrap_or(0);
            let x = app.current_dir.state.select(Some(i));
            // ajkapp.selected_item = String::from(app.current_dir.items[0]);
            // if let Some(thing) = app.current_dir.state.select(i) {
            //     app.selected_item = thing
            // }

            init_current_dir(app);
        }
        Context::GoPrev => {}
    }
}

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
