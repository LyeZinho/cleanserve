use super::manage::{copy_line, run_menu_action, MENU_ITEMS};
use cleanserve_core::RuntimeTelemetry;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{event::DisableMouseCapture, event::EnableMouseCapture, ExecutableCommand};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use ratatui::DefaultTerminal;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct UpDashboardState {
    pub project_name: String,
    pub project_root: PathBuf,
    pub php_version: String,
    pub document_root: String,
    pub url: String,
    pub framework: String,
    pub hot_reload: bool,
    pub telemetry: Arc<RuntimeTelemetry>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusPane {
    Events,
    Requests,
}

struct PanelLayout {
    events: Rect,
    requests: Rect,
}

struct AppUi {
    focus: FocusPane,
    events_offset: usize,
    events_selected: usize,
    requests_offset: usize,
    requests_selected: usize,
    menu_open: bool,
    menu_index: usize,
    toast: Option<(String, Instant)>,
    layout: PanelLayout,
}

impl AppUi {
    fn new() -> Self {
        Self {
            focus: FocusPane::Requests,
            events_offset: 0,
            events_selected: 0,
            requests_offset: 0,
            requests_selected: 0,
            menu_open: false,
            menu_index: 0,
            toast: None,
            layout: PanelLayout {
                events: Rect::default(),
                requests: Rect::default(),
            },
        }
    }

    fn set_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }
}

pub fn run_up_dashboard(state: UpDashboardState) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let mut app = AppUi::new();
    let result = run_loop(&mut terminal, &mut app, &state);
    ratatui::restore();
    let _ = stdout.execute(DisableMouseCapture);
    disable_raw_mode()?;
    stdout.execute(LeaveAlternateScreen)?;
    result
}

fn run_loop(
    terminal: &mut DefaultTerminal,
    app: &mut AppUi,
    state: &UpDashboardState,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app, state))?;

        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.menu_open {
                        if handle_menu_key(app, &key, state) {
                            break;
                        }
                    } else if handle_key(app, &key, state) {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(app, mouse, state);
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if let Some((_, at)) = app.toast {
            if at.elapsed() > Duration::from_secs(3) {
                app.toast = None;
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut AppUi, key: &event::KeyEvent, state: &UpDashboardState) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('c')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            return true;
        }
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {}
        KeyCode::Char('m') => {
            app.menu_open = true;
            app.menu_index = 0;
        }
        KeyCode::Char('c') | KeyCode::Char('y') => {
            copy_selected_request(app, state);
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                FocusPane::Requests => FocusPane::Events,
                FocusPane::Events => FocusPane::Requests,
            };
        }
        KeyCode::Up | KeyCode::Char('k') => scroll_focus(app, state, -1),
        KeyCode::Down | KeyCode::Char('j') => scroll_focus(app, state, 1),
        KeyCode::PageUp => scroll_focus(app, state, -10),
        KeyCode::PageDown => scroll_focus(app, state, 10),
        KeyCode::Home => {
            match app.focus {
                FocusPane::Requests => {
                    app.requests_offset = 0;
                    app.requests_selected = 0;
                }
                FocusPane::Events => {
                    app.events_offset = 0;
                    app.events_selected = 0;
                }
            }
        }
        KeyCode::End => {
            let reqs = request_lines(state);
            let evs = event_lines(state);
            match app.focus {
                FocusPane::Requests => {
                    app.requests_selected = reqs.len().saturating_sub(1);
                    app.requests_offset = app
                        .requests_selected
                        .saturating_sub(visible_rows(app.layout.requests).saturating_sub(1));
                }
                FocusPane::Events => {
                    app.events_selected = evs.len().saturating_sub(1);
                    app.events_offset = app
                        .events_selected
                        .saturating_sub(visible_rows(app.layout.events).saturating_sub(1));
                }
            }
        }
        _ => {}
    }
    false
}

fn handle_menu_key(app: &mut AppUi, key: &event::KeyEvent, state: &UpDashboardState) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('q') => {
            app.menu_open = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.menu_index = app.menu_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.menu_index + 1 < MENU_ITEMS.len() {
                app.menu_index += 1;
            }
        }
        KeyCode::Enter => {
            let selected = selected_request_line(app, state);
            if app.menu_index == MENU_ITEMS.len() - 1 {
                app.menu_open = false;
            } else if let Some(msg) = run_menu_action(
                app.menu_index,
                &state.project_root,
                &state.url,
                selected.as_deref(),
                &state.telemetry,
            ) {
                app.set_toast(msg);
                if app.menu_index >= 4 && app.menu_index <= 5 {
                    app.menu_open = false;
                }
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
        _ => {}
    }
    false
}

fn handle_mouse(app: &mut AppUi, mouse: event::MouseEvent, state: &UpDashboardState) {
    if app.menu_open {
        return;
    }

    let col = mouse.column;
    let row = mouse.row;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if app.layout.requests.contains(ratatui::layout::Position { x: col, y: row }) {
                app.focus = FocusPane::Requests;
                scroll_focus(app, state, -3);
            } else if app.layout.events.contains(ratatui::layout::Position { x: col, y: row }) {
                app.focus = FocusPane::Events;
                scroll_focus(app, state, -3);
            }
        }
        MouseEventKind::ScrollDown => {
            if app.layout.requests.contains(ratatui::layout::Position { x: col, y: row }) {
                app.focus = FocusPane::Requests;
                scroll_focus(app, state, 3);
            } else if app.layout.events.contains(ratatui::layout::Position { x: col, y: row }) {
                app.focus = FocusPane::Events;
                scroll_focus(app, state, 3);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(idx) = row_to_request_index(app, row) {
                let len = request_lines(state).len();
                if idx < len {
                    app.focus = FocusPane::Requests;
                    app.requests_selected = idx;
                    ensure_request_visible(app, state);
                }
            } else if let Some(idx) = row_to_event_index(app, row) {
                let len = event_lines(state).len();
                if idx < len {
                    app.focus = FocusPane::Events;
                    app.events_selected = idx;
                    ensure_event_visible(app, state);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if row_to_request_index(app, row).is_some() {
                copy_selected_request(app, state);
            } else if let Some(line) = event_line_at_row(app, state, row) {
                app.set_toast(copy_line(&line));
            }
        }
        _ => {}
    }
}

fn copy_selected_request(app: &mut AppUi, state: &UpDashboardState) {
    if let Some(line) = selected_request_line(app, state) {
        app.set_toast(copy_line(&line));
    } else {
        app.set_toast("Nothing selected to copy".into());
    }
}

fn selected_request_line(app: &AppUi, state: &UpDashboardState) -> Option<String> {
    let lines = request_lines(state);
    lines.get(app.requests_selected).cloned()
}

fn scroll_focus(app: &mut AppUi, state: &UpDashboardState, delta: i32) {
    match app.focus {
        FocusPane::Requests => {
            let len = request_lines(state).len();
            if len == 0 {
                return;
            }
            let next = (app.requests_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
            app.requests_selected = next;
            ensure_request_visible(app, state);
        }
        FocusPane::Events => {
            let len = event_lines(state).len();
            if len == 0 {
                return;
            }
            let next = (app.events_selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
            app.events_selected = next;
            ensure_event_visible(app, state);
        }
    }
}

fn ensure_request_visible(app: &mut AppUi, state: &UpDashboardState) {
    let visible = visible_rows(app.layout.requests);
    let len = request_lines(state).len();
    if len == 0 {
        return;
    }
    if app.requests_selected < app.requests_offset {
        app.requests_offset = app.requests_selected;
    } else if app.requests_selected >= app.requests_offset + visible {
        app.requests_offset = app.requests_selected + 1 - visible;
    }
    app.requests_offset = app.requests_offset.min(len.saturating_sub(visible));
}

fn ensure_event_visible(app: &mut AppUi, state: &UpDashboardState) {
    let visible = visible_rows(app.layout.events);
    let len = event_lines(state).len();
    if len == 0 {
        return;
    }
    if app.events_selected < app.events_offset {
        app.events_offset = app.events_selected;
    } else if app.events_selected >= app.events_offset + visible {
        app.events_offset = app.events_selected + 1 - visible;
    }
    app.events_offset = app.events_offset.min(len.saturating_sub(visible));
}

fn request_lines(state: &UpDashboardState) -> Vec<String> {
    let mut lines = state.telemetry.snapshot_requests();
    lines.reverse();
    lines
}

fn event_lines(state: &UpDashboardState) -> Vec<String> {
    state.telemetry.snapshot_logs()
}

fn visible_rows(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

fn row_to_request_index(app: &AppUi, row: u16) -> Option<usize> {
    let area = app.layout.requests;
    if row <= area.y || row >= area.y + area.height.saturating_sub(1) {
        return None;
    }
    let row_in_list = (row - area.y - 1) as usize;
    let idx = app.requests_offset + row_in_list;
    Some(idx)
}

fn row_to_event_index(app: &AppUi, row: u16) -> Option<usize> {
    let area = app.layout.events;
    if row <= area.y || row >= area.y + area.height.saturating_sub(1) {
        return None;
    }
    let row_in_list = (row - area.y - 1) as usize;
    Some(app.events_offset + row_in_list)
}

fn event_line_at_row(app: &AppUi, state: &UpDashboardState, row: u16) -> Option<String> {
    row_to_event_index(app, row).and_then(|i| event_lines(state).get(i).cloned())
}

fn truncate_to_width(text: &str, max_cols: u16) -> String {
    let max = max_cols as usize;
    if max < 4 {
        return "…".to_string();
    }
    if text.chars().count() <= max {
        return text.to_string();
    }
    format!(
        "{}…",
        text.chars().take(max.saturating_sub(1)).collect::<String>()
    )
}

fn draw(frame: &mut ratatui::Frame, app: &mut AppUi, state: &UpDashboardState) {
    frame.render_widget(Clear, frame.area());

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " CleanServe ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {} ", state.project_name)),
        Span::styled(
            state.url.clone(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Server "));
    frame.render_widget(header, outer[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(outer[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(4)])
        .split(body[0]);

    let tls_failures = state.telemetry.tls_handshake_failures();
    let status_lines = vec![
        Line::from(format!("Framework: {}", state.framework)),
        Line::from(format!("PHP: {}  |  Root: {}", state.php_version, state.document_root)),
        Line::from(format!(
            "Hot reload: {}  |  HTTP: {}",
            if state.hot_reload { "on" } else { "off" },
            state.telemetry.request_count()
        )),
        if tls_failures > 0 {
            Line::from(Span::styled(
                format!("TLS rejected: {} (use --http-only or trust cert)", tls_failures),
                Style::default().fg(Color::Yellow),
            ))
        } else {
            Line::from("TLS: ok")
        },
    ];
    let status = Paragraph::new(status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Status "),
    );
    frame.render_widget(status, left[0]);

    app.layout.events = left[1];
    draw_scrollable_list(
        frame,
        app,
        left[1],
        FocusPane::Events,
        " Events ",
        &event_lines(state),
        app.events_offset,
        app.events_selected,
    );

    app.layout.requests = body[1];
    let reqs = request_lines(state);
    if !reqs.is_empty() && app.requests_selected >= reqs.len() {
        app.requests_selected = reqs.len() - 1;
    }
    draw_scrollable_list(
        frame,
        app,
        body[1],
        FocusPane::Requests,
        &format!(" Requests ({}) ", reqs.len()),
        &reqs,
        app.requests_offset,
        app.requests_selected,
    );

    let mut footer = " ↑↓/Pg scroll · Tab focus · c copy · m manage · q quit · right-click copy line ";
    if tls_failures > 0 {
        footer = " ↑↓ scroll · m manage · c copy · Trust HTTPS cert or --http-only · q quit ";
    }
    let footer = Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)).block(
        Block::default().borders(Borders::ALL),
    );
    frame.render_widget(footer, outer[2]);

    if app.menu_open {
        draw_manage_menu(frame, app);
    }

    if let Some((ref msg, _)) = app.toast {
        let toast_area = centered_rect(60, 3, frame.area());
        frame.render_widget(Clear, toast_area);
        let p = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Black).bg(Color::Green))
            .block(Block::default().borders(Borders::ALL).title(" Notice "));
        frame.render_widget(p, toast_area);
    }
}

fn draw_scrollable_list(
    frame: &mut ratatui::Frame,
    app: &AppUi,
    area: Rect,
    pane: FocusPane,
    title: &str,
    lines: &[String],
    offset: usize,
    selected: usize,
) {
    let visible = visible_rows(area);
    let width = area.width.saturating_sub(3);
    let focused = app.focus == pane;

    let view: Vec<ListItem> = lines
        .iter()
        .skip(offset)
        .take(visible)
        .enumerate()
        .map(|(i, l)| {
            let global = offset + i;
            let style = if global == selected && focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(truncate_to_width(l, width)).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    if focused {
        if selected >= offset && selected < offset + visible {
            list_state.select(Some(selected - offset));
        }
    }

    let block = Block::default().borders(Borders::ALL).title(title);
    let list = List::new(view).block(block).highlight_symbol("▸ ");
    frame.render_stateful_widget(list, area, &mut list_state);

    if lines.len() > visible {
        let mut scroll = ScrollbarState::new(lines.len().saturating_sub(visible)).position(offset);
        let scroll_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            scroll_area,
            &mut scroll,
        );
    }
}

fn draw_manage_menu(frame: &mut ratatui::Frame, app: &AppUi) {
    let area = centered_rect(56, MENU_ITEMS.len() as u16 + 4, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" Manage (Enter run · Esc close) ")
            .style(Style::default().fg(Color::Cyan)),
        area,
    );

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = if i == app.menu_index {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(*label).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.menu_index));
    let list = List::new(items);
    frame.render_stateful_widget(list, inner, &mut state);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(percent_x),
            Constraint::Fill(1),
        ])
        .split(popup_layout[1])[1]
}
