use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{palette::tailwind, Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame,
};

use crate::{
    state::{App, State},
    tui,
};

const ITEM_HEIGHT: usize = 4;
const INFO_TEXT: &str = "(q) Quit (j/k) Up/Down (r) Refresh (space) Select (-) Back";
const TITLE: &str = "APPS";

#[derive(Clone)]
pub struct AppsTableView {
    state: TableState,
    data: Vec<App>,
    max_item_lens: (u16, u16),
    scroll_state: ScrollbarState,
    theme: TableColors,
}

impl AppsTableView {
    pub fn new(state: &State) -> Self {
        let max_name_len = state
            .keys()
            .iter()
            .map(|name| name.len())
            .max()
            .unwrap_or(0) as u16;
        let max_cluster_len = state
            .values()
            .iter()
            .map(|app| app.cluster.len())
            .max()
            .unwrap_or(0) as u16;

        let scroll_state = match state.keys().len() {
            0 => ScrollbarState::default(),
            n => ScrollbarState::new((n - 1) * ITEM_HEIGHT),
        };

        Self {
            state: TableState::default().with_selected(0),
            max_item_lens: (max_name_len, max_cluster_len),
            scroll_state,
            theme: TableColors::new(tui::THEME),
            data: state.values().into_iter().cloned().collect(), // Vec<&'a App> ?
        }
    }

    pub fn update(&mut self, state: &mut State) {
        state.update_apps();

        let max_name_len = state
            .keys()
            .iter()
            .map(|name| name.len())
            .max()
            .unwrap_or(0) as u16;

        let max_cluster_len = state
            .values()
            .iter()
            .map(|app| app.cluster.len())
            .max()
            .unwrap_or(0) as u16;

        self.max_item_lens = (max_name_len, max_cluster_len);
        self.data = state.values().into_iter().cloned().collect();
        self.scroll_state = match self.data.len() {
            0 => ScrollbarState::default(),
            n => ScrollbarState::new((n - 1) * ITEM_HEIGHT),
        };
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn down(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                match self.data.len() {
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
                match self.data.len() {
                    0 | 1 => 0,               // no scroll
                    len if i == 0 => len - 1, // wrap-around
                    _ => i - 1,               // prev scroll
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn selected_name(&self) -> String {
        let idx = self.state.selected().unwrap_or(0);
        self.data[idx].name.clone()
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
    }
}

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

impl AppsTableView {
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

    fn render_table(&mut self, frame: &mut Frame, layout: Rect) {
        let header_style = Style::default()
            .fg(self.theme.header_fg)
            .bg(self.theme.header_bg);

        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.theme.selected_style_fg);

        let header = ["POD", "CLUSTER"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.data.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.theme.normal_row,
                _ => self.theme.alt_row,
            };

            let columns = [&data.name, &data.cluster];
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
                Constraint::Length(self.max_item_lens.0 + 1),
                Constraint::Min(self.max_item_lens.1 + 1),
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
