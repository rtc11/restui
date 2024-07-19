use std::panic;

use arg::ArgMethod;
use color_eyre::eyre::{self, Context};
use ratatui::{
    backend::Backend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    Terminal,
};
use state::State;
use tui::{View, TUI};
use views::requests::{Direction, Editing, RequestMode};

pub mod arg;
pub mod client;
pub mod k8s;
pub mod oauth2;
pub mod state;
pub mod tui;
pub mod util;
pub mod views;

pub const NAMESPACE: &str = "helved";

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    install_hooks()?;
    let terminal = tui::init()?;
    let state = State::load();
    let tui = TUI::new(state);
    run(tui, terminal).wrap_err("run failed")?;

    tui::restore()?;

    Ok(())
}

impl From<arg::ArgMethod> for state::Method {
    fn from(value: arg::ArgMethod) -> Self {
        match value {
            ArgMethod::Get => state::Method::Get,
            ArgMethod::Post => state::Method::Post,
            ArgMethod::Put => state::Method::Put,
            ArgMethod::Patch => state::Method::Patch,
            ArgMethod::Delete => state::Method::Delete,
        }
    }
}

#[allow(dead_code)]
fn run(mut tui: tui::TUI, mut term: Terminal<impl Backend>) -> color_eyre::Result<()> {
    // let mut last_key: KeyCode = KeyCode::Null;

    loop {
        term.draw(|frame| {
            match &mut tui.view {
                View::Apps(view) => view.render(frame),
                View::Ingresses(view) => view.render(frame),
                View::Requests(view) => view.render(frame),
            };
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match &mut tui.view {
                    View::Apps(view) => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('j') | KeyCode::Down => view.down(),
                        KeyCode::Char('k') | KeyCode::Up => view.up(),
                        KeyCode::Char(' ') | KeyCode::Enter => tui.enter(),
                        KeyCode::Char('-') | KeyCode::Backspace => tui.back(),
                        KeyCode::Char('r') => tui.refresh(),
                        _ => {}
                    },
                    View::Ingresses(view) => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('j') | KeyCode::Down => view.down(),
                        KeyCode::Char('k') | KeyCode::Up => view.up(),
                        KeyCode::Char(' ') | KeyCode::Enter => tui.enter(),
                        KeyCode::Char('-') | KeyCode::Backspace => tui.back(),
                        KeyCode::Char('r') => tui.refresh(),
                        _ => {}
                    },
                    View::Requests(view) => match &mut view.mode {
                        RequestMode::Normal => match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('j') | KeyCode::Down => view.down(),
                            KeyCode::Char('k') | KeyCode::Up => view.up(),
                            KeyCode::Char('-') | KeyCode::Backspace => tui.back(),
                            KeyCode::Char('e') => view.edit(),
                            KeyCode::Char('n') => view.new_request(),
                            _ => {},
                        },
                        RequestMode::Insert(edit, _) => match key.code {
                            KeyCode::Esc => view.save(&mut tui.state),
                            KeyCode::Tab => view.next_edit(&mut tui.state),
                            KeyCode::Left => edit.move_cursor(Direction::Left, 1),
                            KeyCode::Right => edit.move_cursor(Direction::Right, 1),
                            KeyCode::Up => edit.move_cursor(Direction::Up, 1),
                            KeyCode::Down => edit.move_cursor(Direction::Down, 1),
                            KeyCode::Char(n) => edit.add_char(n),
                            KeyCode::Enter => edit.new_line(),
                            KeyCode::Backspace => edit.del_char(),
                            KeyCode::End => edit.move_cursor(Direction::Right, usize::MAX),
                            KeyCode::Home => edit.move_cursor(Direction::Left, usize::MAX),
                            _ => {},
                        }
                        
                    },
                }
            }
        }
    }
    Ok(())
}

pub fn install_hooks() -> color_eyre::Result<()> {
    let hook_builder = color_eyre::config::HookBuilder::default();
    let (panic_hook, eyre_hook) = hook_builder.into_hooks();

    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tui::restore().unwrap();
        panic_hook(panic_info);
    }));

    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(move |error| {
        tui::restore().unwrap();
        eyre_hook(error)
    }))?;

    Ok(())
}
