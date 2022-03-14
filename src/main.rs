#![allow(deprecated)]
use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};

use dirs;

#[derive(Parser)]
#[clap(name = "Rusty Ranger")]
#[clap(author = "Morris Alromhein")]
#[clap(version = "1.0")]
#[clap(about = "Ranger style file explorer written in Rust")]
struct Args {
    #[clap(short = 'f', long)]
    filename: Option<String>,

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
}
struct App<'a> {
    /// Basic items.
    show_hidden: bool,
    hovered_index: i32,

    /// Files in the Dirs
    current_dir_vec: Vec<String>,
    previous_dir_vec: Vec<String>,
    next_dir_vec: Vec<String>,

    /// Using PathBuff object to grab files
    pwd: std::path::PathBuf,

    current_dir_list: StatefulList<ListItem<'a>>,
}

impl<'a> Default for App<'a> {
    fn default() -> App<'a> {
        App {
            current_dir_list: StatefulList::with_items(Vec::new()),
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

impl App<'_> {
    fn update_list(&mut self) {
        self.current_dir_list = StatefulList::with_items(Vec::new());
        for file_name in self.current_dir_vec.clone() {
            let file_is_hidden = match file_name.chars().next() {
                Some('.') => true,
                Some(_) => false,
                None => false,
            };

            if !file_is_hidden || self.show_hidden {
                self.current_dir_list.items.push(ListItem::new(file_name));
            }
            self.current_dir_list.state.select(Some(0));
        }
    }
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
        self.hovered_index = (self.hovered_index - 1).rem_euclid(self.current_dir_vec.len() as i32);
    }

    fn next(&mut self) {
        if self.current_dir_vec.len() == 0 {
            return;
        }
        self.hovered_index = (self.hovered_index + 1).rem_euclid(self.current_dir_vec.len() as i32);
    }

    fn into_dir(&mut self) {
        self.previous_dir_vec = Vec::new();
        let mut pp = self.pwd.clone();
        self.previous_dir_vec = self.get_files_as_vec(&pp);
        pp.push(self.current_dir_vec[self.hovered_index as usize].clone());
        self.pwd = pp.clone();
        self.current_dir_vec = Vec::new();
        self.current_dir_vec = self.get_files_as_vec(&pp);
        if self.current_dir_vec.len() == 0 {
            self.current_dir_vec.push("Empty Dir...".to_string());
        }
        self.hovered_index = 0;
        self.current_dir_vec.sort();
        self.previous_dir_vec.sort();
        self.next_dir_vec.sort();
        self.update_list();
        // self.next_dir_vec = vec![
        //     self.pwd.to_str().unwrap().to_string(),
        //     pp.to_str().unwrap().to_string(),
        // ]
    }

    fn out_dir(&mut self) {
        self.next_dir_vec = self.current_dir_vec.clone();
        self.current_dir_vec = self.previous_dir_vec.clone();
        self.current_dir_vec.sort();
        self.previous_dir_vec.sort();
        self.next_dir_vec.sort();
        let x = self.pwd.pop();
        let mut pwd = self.pwd.clone();
        if x {
            pwd.pop();
            self.previous_dir_vec = self.get_files_as_vec(&pwd);
        } else {
            self.previous_dir_vec = Vec::new();
        }
        self.hovered_index = 0;
        self.update_list();
    }

    fn hover(&mut self) {
        let p = match self.pwd.parent() {
            Some(_) => true,
            None => false,
        };
        if !p {
            self.previous_dir_vec = Vec::new();
        }
        let mut t = PathBuf::new();
        t.push(&self.pwd);
        t.push(&self.current_dir_vec[self.hovered_index as usize]);
        // let t = Path::new(&self.current_dir_vec[self.hovered_index as usize]);
        if let Ok(metadata) = t.metadata() {
            if metadata.is_dir() {
                self.next_dir_vec = self.get_files_as_vec(&t);
            } else {
                self.next_dir_vec = Vec::new();

                let data = fs::read_to_string(t).unwrap_or("Error Loading File".to_string());
                self.next_dir_vec.push(data);
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let mut app = App::default();
    let args = Args::parse();

    let custom_dir = match args.filename.clone() {
        Some(_) => true,
        None => false,
    };
    if custom_dir {
        app.pwd.push(args.filename.unwrap());
    } else {
        app.pwd = dirs::home_dir().unwrap();
    }
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
    app.update_list();
    loop {
        app.hover();
        terminal.draw(|f| ui(f, &mut app))?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Down => app.next(),
                KeyCode::Char('j') => {
                    app.current_dir_list.next();
                    app.next();
                    // app.hover();
                }
                KeyCode::Up => app.previous(),
                KeyCode::Char('k') => {
                    app.current_dir_list.previous();
                    app.previous();
                    // app.hover();
                }
                KeyCode::Char('l') => {
                    let mut pp = app.pwd.clone();
                    pp.push(Path::new(&app.current_dir_vec[app.hovered_index as usize]));
                    if let Ok(metadata) = pp.metadata() {
                        if metadata.is_dir() {
                            app.into_dir();
                        }
                    }
                }
                KeyCode::Char('h') => {
                    let p = match app.pwd.parent() {
                        Some(_) => true,
                        None => false,
                    };
                    if p {
                        app.out_dir();
                    } else {
                    }
                }
                KeyCode::Char('s') => {
                    app.show_hidden = !app.show_hidden;
                    let pp = app.pwd.clone();
                    app.current_dir_vec = app.get_files_as_vec(&pp);
                    app.current_dir_vec.sort();
                    app.update_list();
                    app.previous_dir_vec =
                        app.get_files_as_vec(pp.parent().unwrap_or(&PathBuf::new()));
                    app.hover();
                    continue;
                }
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

    let items = app.current_dir_list.items.to_owned();
    let main_block = List::new(items)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default())
        .highlight_symbol(">> ");
    f.render_stateful_widget(main_block, chunks[1], &mut app.current_dir_list.state);

    // let items = app.current_dir_list.items.to_owned();
    // let mut items: Vec<ListItem> = Vec::new();
    // for file_name in app.current_dir_vec.clone() {
    //     items.push(ListItem::new(file_name));
    // }
    //
    // let mut items: Vec<ListItem> = Vec::new();
    // for file_name in app.current_dir_vec.clone() {
    //     let file_is_hidden = match file_name.chars().next() {
    //         Some('.') => true,
    //         Some(_) => false,
    //         None => false,
    //     };
    //     if !file_is_hidden || app.show_hidden {
    //         items.push(ListItem::new(file_name))
    //     }
    // }
    // let main_block = List::new(items)
    //     .style(Style::default().fg(Color::White))
    //     .block(Block::default().borders(Borders::ALL));
    // f.render_widget(main_block, chunks[1]);

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

// TODO Wrap seperate windows in TOKIO async
// TODO System Calls... delete copy paste rename ..
// TODO Add file metadata to bottom drwxr-xr-x and bunch more stuff
// TODO Split this file into multiple files
