use std::io::{self, stdout};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    style::palette::tailwind::{Palette, BLUE, EMERALD, INDIGO, RED},
    Frame, Terminal,
};

use crate::{
    state::{App, State},
    views::{apps::AppsTableView, hosts::IngressView, requests::RequestView},
};

pub fn init() -> io::Result<Terminal<impl Backend>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore() -> io::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

const THEMES: [Palette; 4] = [BLUE, EMERALD, INDIGO, RED];
pub const THEME: &Palette = &THEMES[1];

pub fn layout(tui: &mut TUI, frame: &mut Frame) {
    match &mut tui.view {
        View::Apps(view) => view.render(frame),
        View::Ingresses(view) => view.render(frame),
        View::Requests(view) => view.render(frame),
    };
}

#[derive(Clone)]
pub enum View {
    Apps(AppsTableView),
    Ingresses(IngressView),
    Requests(RequestView),
}

#[derive(Clone)]
pub struct TUI {
    pub view: View,
    pub state: State,
}

impl Drop for TUI {
    fn drop(&mut self) {
        self.state.save();
    }
}

impl TUI {
    pub fn new(state: State) -> Self {
        Self {
            view: View::Apps(AppsTableView::new(&state)),
            state,
        }
    }

    pub fn get_app_by_name(&self, name: &str) -> Option<App> {
        self.state.get(name).cloned()
    }

    pub fn select_apps(&mut self) {
        self.view = View::Apps(AppsTableView::new(&self.state))
    }

    pub fn select_ingresses(&mut self, app: &App) {
        self.view = View::Ingresses(IngressView::new(app))
    }

    pub fn select_requests(&mut self, app: &App) {
        self.view = View::Requests(RequestView::new(app))
    }

    pub fn enter(&mut self) {
        match &mut self.view {
            View::Apps(view) => {
                let name = view.selected_name();
                let app = self.get_app_by_name(&name).unwrap();
                self.select_ingresses(&app);
            }
            View::Ingresses(view) => {
                let app = view.nais_app();
                self.select_requests(&app);
            }
            View::Requests(_) => {}
        }
    }

    pub fn add_random_request(&mut self) {
        if let View::Requests(view) = &mut self.view {
            view.add_random_request(&mut self.state);
        }
    }

    pub fn back(&mut self) {
        match &mut self.view {
            View::Apps(_) => {}
            View::Ingresses(_) => self.select_apps(),
            View::Requests(view) => {
                let app = view.nais_app();
                self.select_ingresses(&app);
            }
        }
    }

    pub fn refresh(&mut self) {
        match &mut self.view {
            View::Apps(view) => view.update(&mut self.state),
            View::Ingresses(view) => view.update(&mut self.state),
            _ => {}
        };

        self.state.save();
    }
}
