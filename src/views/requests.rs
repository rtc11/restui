use std::collections::BTreeSet;

use ratatui::layout::Layout;
use ratatui::style::palette::tailwind::Palette;
use ratatui::text::Span;
use ratatui::widgets::{Clear, List, ListItem, ScrollbarState, TableState};

use ratatui::{
    layout::{Constraint, Margin, Rect},
    style::{palette::tailwind, Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        Table,
    },
    Frame,
};

use crate::state::{App, Header, Method, Request, State};
use crate::tui;

const INFO_TEXT: &str = "(q)uit (n)ew (e)dit (j/k) up/down (-) back ( ) select";
const TITLE: &str = "REQUESTS";
const ITEM_HEIGHT: usize = 4;

#[derive(Clone)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row: Color,
    alt_row: Color,
    footer_boarder: Color,
    header_boarder: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row: tailwind::SLATE.c950,
            alt_row: tailwind::SLATE.c900,
            footer_boarder: color.c400,
            header_boarder: color.c400,
        }
    }
}

#[derive(Clone)]
pub struct RequestView {
    state: TableState,
    pub data: App,
    max_len: (u16, u16, u16),
    scroll_state: ScrollbarState,
    theme: TableColors,
    pub mode: RequestMode,
    pub editables: Vec<Editable>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum RequestMode {
    Normal,
    Insert(Editable, Field),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Field {
    Desc,
    Path,
    Headers,
    Body,
}

impl RequestView {
    pub fn new(app: &App) -> Self {
        let scroll_state = match app.requests.len() {
            0 => ScrollbarState::default(),
            n => ScrollbarState::new((n - 1) * ITEM_HEIGHT),
        };

        Self {
            state: TableState::default().with_selected(0),
            scroll_state,
            theme: TableColors::new(tui::THEME),
            data: app.clone(),
            max_len: (30, 30, 30),
            mode: RequestMode::Normal,
            editables: vec![],
        }
    }

    pub fn add_random_request(&mut self, state: &mut State) {
        let random_req = Request::new(
            Method::Get,
            "/random",
            vec![Header::new(
                "Content-Type".into(),
                "application/jsonish".into(),
            )],
            "{ body }",
        );

        self.data.requests.insert(random_req);
        state.insert(self.data.clone());
        state.save();
    }

    pub fn update(&mut self, state: &mut State) {
        self.data = state
            .get(&self.data.name)
            .expect("app in requestview")
            .clone();

        self.scroll_state = match self.data.requests.len() {
            0 => ScrollbarState::default(),
            n => ScrollbarState::new((n - 1) * ITEM_HEIGHT),
        };
    }

    pub fn nais_app(&self) -> App {
        self.data.clone()
    }

    pub fn size(&self) -> usize {
        self.data.requests.len()
    }

    pub fn down(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                match self.data.requests.len() {
                    0 | 1 => 0,               // no scroll
                    len if i >= len - 1 => 0, // wrap-around
                    _ => i + 1,               // next scroll
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn up(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let up = i.saturating_sub(1);
                up.clamp(0, self.data.requests.len() - 1)
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.size());

        self.render_header(frame, layout[0]);
        self.render_table(frame, layout[1]);
        self.render_scrollbar(frame, layout[1]);
        self.render_footer(frame, layout[2]);
        self.render_editor(frame);
    }

    pub fn edit(&mut self) {
        if self.mode == RequestMode::Normal {
            self.editables = self.editables();
            self.mode = RequestMode::Insert(self.editables[0].clone(), Field::Desc);
        }
    }

    pub fn new_request(&mut self) {
        if self.mode == RequestMode::Normal {
            let mut req = Request::default();
            let id = self.data.requests.len() + 1;
            req.id = id as u64;
            self.data.requests.insert(req);
        }
    }

    fn editables(&mut self) -> Vec<Editable> {
        let idx = self.state.selected().unwrap_or(0);
        let req = &mut self.data.requests.clone().into_iter().collect::<Vec<_>>()[idx];
        let e_desc = Editable::new(vec![&req.desc]);
        let e_path = Editable::new(vec![&req.path]);
        let e_head = Editable::from(&req.headers);
        let e_body = Editable::new(vec![req.body.clone()]);
        vec![e_desc, e_path, e_head, e_body]
    }

    pub fn next_edit(&mut self, state: &mut State) {
        let mode = self.mode.clone();
        self.save(state);
        self.mode = mode;
        self.editables = self.editables();

        if let RequestMode::Insert(_, field) = &self.mode {
            match field {
                Field::Desc => {
                    self.mode = RequestMode::Insert(self.editables[1].clone(), Field::Path)
                },
                Field::Path => {
                    self.mode = RequestMode::Insert(self.editables[2].clone(), Field::Headers)
                }
                Field::Headers => {
                    self.mode = RequestMode::Insert(self.editables[3].clone(), Field::Body)
                }
                Field::Body => {
                    self.mode = RequestMode::Insert(self.editables[0].clone(), Field::Desc)
                }
            }
        }
    }

    pub fn save(&mut self, state: &mut State) {
        if let RequestMode::Insert(editable, field) = &self.mode {
            let idx = self.state.selected().unwrap_or(0);
            let req = &mut self.data.requests.clone().into_iter().collect::<Vec<_>>()[idx];

            match field {
                Field::Desc => {
                    let desc = editable.input.clone().into_iter().next().unwrap_or_default(); 
                    req.desc = desc;
                },
                Field::Path => {
                    let path = editable.input.clone().into_iter().next().unwrap_or_default(); 
                    req.path = path;
                },
                Field::Headers => {
                    let headers = BTreeSet::<Header>::from(editable.clone());
                    req.headers = headers;
                }
                Field::Body => {
                    let body = editable.input.clone().into_iter().next().unwrap_or_default(); 
                    req.body = body;
                },
            }
            self.mode = RequestMode::Normal;
            self.data.requests.insert(req.clone());
            self.data.add_request(req.clone());
            state.insert(self.data.clone());
            state.save();
        }
    }

    pub fn render_editor(&mut self, frame: &mut Frame) {
        if let RequestMode::Insert(editable, field) = &mut self.mode {
            let idx = self.state.selected().unwrap_or(0);
            let req = self.data.requests.iter().collect::<Vec<_>>()[idx];

            let area = centered_rect(80, 40, frame.size());

            let layout = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Fill(1),
            ]);

            frame.render_widget(Clear, area); // clear screen under the popup
            let [area_host, area_desc, area_path, area_head, area_body] = layout.areas(area);
            let idx = self.state.selected().unwrap_or(0);

            // HOST
            let block = Block::bordered()
                .title("Host")
                .border_style(PALETTES[idx].c700);
            let host = self
                .data
                .hosts
                .first()
                .expect("app.host in requests view");
            let host = Paragraph::new(host.to_owned()).block(block);
            frame.render_widget(host, area_host);

            // DESC
            let block = Block::bordered()
                .title("Description")
                .border_style(PALETTES[idx].c700);
            let desc = if field == &Field::Desc {
                editable
                    .input
                    .first()
                    .expect("editable.input.first in render_editor")
            } else {
                &req.desc
            };
            let desc = Paragraph::new(desc.to_owned()).block(block);
            frame.render_widget(desc, area_desc);

            // PATH
            let block = Block::bordered()
                .title("Path")
                .border_style(PALETTES[idx].c700);
            let path = if field == &Field::Path {
                editable
                    .input
                    .clone()
                    .into_iter()
                    .next()
                    .unwrap_or("unknown".to_string())
            } else {
                req.path.clone()
            };
            let path = Paragraph::new(path).block(block);
            frame.render_widget(path, area_path);

            // HEADER
            let headers: Vec<ListItem> = if field == &Field::Headers {
                editable
                    .input
                    .iter()
                    .map(|m| ListItem::new(Line::from(Span::raw(m))))
                    .collect()
            } else {
                self.data.requests.iter().collect::<Vec<_>>()[idx]
                    .headers
                    .iter()
                    .map(|h| format!("{:<15} {}", h.key, h.value))
                    .map(|m| ListItem::new(Line::from(Span::raw(m))))
                    .collect()
            };
            let block = Block::bordered()
                .title("Headers")
                .border_style(PALETTES[idx].c700);
            let headers = List::new(headers).block(block);
            frame.render_widget(headers, area_head);

            // BODY
            let body: Vec<ListItem> = if field == &Field::Body {
                editable
                    .input
                    .iter()
                    .map(|m| ListItem::new(Line::from(Span::raw(m))))
                    .collect()
            } else {
                req.body
                    .split('\n')
                    .map(|m| ListItem::new(Line::from(Span::raw(m))).clone())
                    .collect()
            };
            let block = Block::bordered()
                .title("Body")
                .border_style(PALETTES[idx].c700);
            let body = List::new(body).block(block);
            frame.render_widget(body, area_body);
            // let body = Paragraph::new(body).block(block);
            // frame.render_widget(body, area_body);

            // CURSOR
            let cursor_area = match field {
                Field::Desc => area_desc,
                Field::Path => area_path,
                Field::Headers => area_head,
                Field::Body => area_body,
            };
            frame.set_cursor(
                cursor_area.x + editable.x as u16 + 1,
                cursor_area.y + editable.y as u16 + 1,
            );
        }
    }
}

const PALETTES: [Palette; 7] = [
    tailwind::RED,
    tailwind::YELLOW,
    tailwind::GREEN,
    tailwind::BLUE,
    tailwind::INDIGO,
    tailwind::PURPLE,
    tailwind::PINK,
];

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_lay = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_lay[1])[1]
}

impl RequestView {
    fn render_table(&mut self, frame: &mut Frame, layout: Rect) {
        let header_style = Style::default()
            .fg(self.theme.header_fg)
            .bg(self.theme.header_bg);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.theme.selected_style_fg);

        let header = ["METHOD", "PATH", "DESC"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.data.requests.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.theme.normal_row,
                _ => self.theme.alt_row,
            };

            let method = format!("{}", &data.method);
            let columns = vec![&method, &data.path, &data.desc];

            columns
                .into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.theme.row_fg).bg(color))
                .height(4)
        });

        let bar = " █ ";
        let t = Table::new(
            rows,
            [
                Constraint::Length(self.max_len.0 + 1),
                Constraint::Length(self.max_len.1 + 1),
                Constraint::Length(self.max_len.2 + 1),
            ],
        )
        .header(header)
        .highlight_style(selected_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(self.theme.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(t, layout, &mut self.state)
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, layout: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            layout.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_header(&mut self, frame: &mut Frame, layout: Rect) {
        let info_header = Paragraph::new(Line::from(TITLE))
            .style(
                Style::new()
                    .fg(self.theme.header_boarder)
                    .bg(self.theme.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.theme.header_boarder)),
            );
        frame.render_widget(info_header, layout);
    }

    fn render_footer(&mut self, frame: &mut Frame, layout: Rect) {
        let info_footer = Paragraph::new(Line::from(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.theme.footer_boarder)
                    .bg(self.theme.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.theme.footer_boarder)),
            );
        frame.render_widget(info_footer, layout);
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Editable {
    pub input: Vec<String>,
    x: usize,
    y: usize,
}

impl Editable {
    pub fn new<T>(input: Vec<T>) -> Self
    where
        T: ToString,
    {
        Self {
            input: input.into_iter().map(|i| i.to_string()).collect(),
            x: 0,
            y: 0,
        }
    }
}

impl From<&BTreeSet<Header>> for Editable {
    fn from(value: &BTreeSet<Header>) -> Self {
        Self {
            input: value
                .iter()
                .map(|h| format!("{:<15} {}", h.key, h.value))
                .collect(),
            x: 0,
            y: 0,
        }
    }
}

impl From<Editable> for BTreeSet<Header> {
    fn from(value: Editable) -> Self {
        value
            .input
            .iter()
            .map(|h| {
                let mut split = h.split_whitespace();
                Header::new(
                    split.next().unwrap_or_default().to_string(),
                    split.collect::<Vec<&str>>().join(" "),
                )
            })
            .collect()
    }
}

impl Editing for Editable {
    fn new_line(&mut self) {
        // caret is at the end of the line
        if self.x == self.input[self.y].len() {
            self.input.insert(self.y + 1, String::new());
            self.y += 1;
            self.x = 0;
        }
        // caret is at the start of the line with content
        else if self.x == 0 && !self.input.is_empty() {
            let prev = self.input[self.y].clone();
            self.input.remove(self.y);
            self.input.insert(self.y, String::new());
            self.input.insert(self.y + 1, prev);
            self.y += 1;
            self.x = 0;
        }
        // caret is in the middle of the line
        else if self.x > 0 && self.x < self.input[self.y].len() {
            let lpos: String = self.input[self.y].chars().take(self.x).collect();
            let rpos: String = self.input[self.y].chars().skip(self.x).collect();
            self.input.remove(self.y);
            self.input.insert(self.y, lpos);
            self.input.insert(self.y + 1, rpos);
            self.y += 1;
            self.x = 0;
        }
    }

    fn move_cursor(&mut self, dir: Direction, steps: usize) {
        match dir {
            Direction::Up => {
                let up = self.y.saturating_sub(steps);
                self.y = up.clamp(0, self.input.len());
                self.x = self.x.clamp(0, self.input[self.y].chars().count());
            }
            Direction::Down => {
                let down = self.y.saturating_add(steps);
                self.y = down.clamp(0, self.input.len() - 1);
                self.x = self.x.clamp(0, self.input[self.y].chars().count());
            }
            Direction::Left => {
                let left = self.x.saturating_sub(steps);
                self.x = left.clamp(0, self.input[self.y].chars().count());
            }
            Direction::Right => {
                let right = self.x.saturating_add(steps);
                self.x = right.clamp(0, self.input[self.y].chars().count())
            }
        }
    }

    fn add_char(&mut self, c: char) {
        if self.input.is_empty() {
            self.input.push(String::new());
        }
        let idx = self.input[self.y]
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.x)
            .unwrap_or(self.input[self.y].len());

        self.input[self.y].insert(idx, c);
        self.move_cursor(Direction::Right, 1);
    }

    fn del_char(&mut self) {
        match (self.x, self.y) {
            // delete prev char
            (x, y) if x != 0 => {
                let lpos = self.input[y].chars().take(x - 1);
                let rpos = self.input[y].chars().skip(x);
                self.input[y] = lpos.chain(rpos).collect();
                self.move_cursor(Direction::Left, 1);
            }
            // delete to prev row
            (x, y) if x == 0 && y > 0 => {
                let prev_y_input = self.input[y - 1].clone();
                self.x = prev_y_input.len();
                self.move_cursor(Direction::Up, 1);
                self.input[y - 1] = prev_y_input + &self.input[y];
                self.input.remove(y);
            }
            _ => {}
        }
    }
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub trait Editing {
    fn new_line(&mut self);
    fn move_cursor(&mut self, dir: Direction, steps: usize);
    fn add_char(&mut self, c: char);
    fn del_char(&mut self);
}
