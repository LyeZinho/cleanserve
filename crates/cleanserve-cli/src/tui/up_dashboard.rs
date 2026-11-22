//! `cleanserve up` dashboard: one calm screen with the server URL and a live request log.
//!
//! ```text
//!  ◉ CleanServe  my-app                                   ● running  12:41
//!
//!  ➜  Local   https://localhost:8080
//!     PHP 8.4 · public/ · hot reload on
//!
//!  ── Requests · 3 ─────────────────────────────────────────────────────
//!   20:55:14  GET      200  /                                       12ms
//!   20:55:15  PUT      404  /api/items/1                             3ms
//!  q quit  ↑↓ scroll  c copy  o open  l logs  m more
//! ```

use super::manage::{copy_line, open_url, run_menu_action, MENU_ITEMS};
use chrono::{DateTime, Local};
use cleanserve_core::{RequestEntry, RuntimeTelemetry};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use crossterm::{event::DisableMouseCapture, event::EnableMouseCapture, ExecutableCommand};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const LOGS_HEIGHT: u16 = 8;

pub struct UpDashboardState {
    pub project_name: String,
    pub project_root: PathBuf,
    pub php_version: String,
    pub document_root: String,
    pub url: String,
    pub framework: String,
    pub hot_reload: bool,
    pub telemetry: Arc<RuntimeTelemetry>,
    /// Set by the caller to close the dashboard (signal received, server failed).
    pub shutdown: Arc<AtomicBool>,
}

/// Selection + scrolling of a list that follows its tail until the user moves up.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ListView {
    selected: usize,
    offset: usize,
    follow: bool,
}

impl ListView {
    fn new() -> Self {
        Self { selected: 0, offset: 0, follow: true }
    }

    /// Re-clamp after the list length or the viewport height changed.
    fn sync(&mut self, len: usize, visible: usize) {
        if len == 0 {
            *self = Self::new();
            return;
        }
        if self.follow {
            self.selected = len - 1;
        }
        self.selected = self.selected.min(len - 1);
        let visible = visible.max(1);
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + visible {
            self.offset = self.selected + 1 - visible;
        }
        self.offset = self.offset.min(len.saturating_sub(visible));
    }

    fn move_by(&mut self, delta: i32, len: usize) {
        if len == 0 {
            return;
        }
        let next = (self.selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.selected = next;
        self.follow = next == len - 1;
    }

    fn first(&mut self) {
        self.selected = 0;
        self.follow = false;
    }

    fn last(&mut self, len: usize) {
        self.selected = len.saturating_sub(1);
        self.follow = true;
    }

    fn select(&mut self, index: usize, len: usize) {
        if index < len {
            self.selected = index;
            self.follow = index == len - 1;
        }
    }
}

struct AppUi {
    view: ListView,
    show_logs: bool,
    menu_open: bool,
    menu_index: usize,
    toast: Option<(String, Instant)>,
    /// Where the request rows were drawn last frame (for mouse hit-testing and paging).
    list_area: Rect,
    started: Instant,
}

impl AppUi {
    fn new() -> Self {
        Self {
            view: ListView::new(),
            show_logs: false,
            menu_open: false,
            menu_index: 0,
            toast: None,
            list_area: Rect::default(),
            started: Instant::now(),
        }
    }

    fn set_toast(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }
}

pub fn run_up_dashboard(state: UpDashboardState) -> anyhow::Result<()> {
    // `ratatui::init` enables raw mode + the alternate screen and installs a panic hook that
    // restores the terminal.
    let mut terminal = ratatui::init();
    let mut stdout = stdout();
    let _ = stdout.execute(EnableMouseCapture);
    let mut app = AppUi::new();
    let result = run_loop(&mut terminal, &mut app, &state);
    let _ = stdout.execute(DisableMouseCapture);
    ratatui::restore();
    result
}

fn run_loop(
    terminal: &mut DefaultTerminal,
    app: &mut AppUi,
    state: &UpDashboardState,
) -> anyhow::Result<()> {
    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            break;
        }
        terminal.draw(|frame| draw(frame, app, state))?;

        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                // Windows also reports key releases; act on presses only.
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    let quit = if app.menu_open {
                        handle_menu_key(app, &key, state)
                    } else {
                        handle_key(app, &key, state)
                    };
                    if quit {
                        break;
                    }
                }
                Event::Mouse(mouse) => handle_mouse(app, mouse, state),
                _ => {}
            }
        }

        if matches!(app.toast, Some((_, at)) if at.elapsed() > Duration::from_secs(3)) {
            app.toast = None;
        }
    }
    Ok(())
}

fn requests(state: &UpDashboardState) -> Vec<RequestEntry> {
    state.telemetry.snapshot_requests()
}

fn request_text(e: &RequestEntry) -> String {
    format!("{} {} {}", e.method, e.path, e.status)
}

fn selected_request_text(app: &AppUi, state: &UpDashboardState) -> Option<String> {
    requests(state).get(app.view.selected).map(request_text)
}

fn handle_key(app: &mut AppUi, key: &event::KeyEvent, state: &UpDashboardState) -> bool {
    let len = requests(state).len();
    let page = app.list_area.height.saturating_sub(1).max(1) as i32;
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
        KeyCode::Char('q') => return true,
        KeyCode::Char('m') => {
            app.menu_open = true;
            app.menu_index = 0;
        }
        KeyCode::Char('l') => app.show_logs = !app.show_logs,
        KeyCode::Char('o') => app.set_toast(open_url(&state.url)),
        KeyCode::Char('c') | KeyCode::Char('y') => match selected_request_text(app, state) {
            Some(line) => app.set_toast(copy_line(&line)),
            None => app.set_toast("Nothing selected to copy".into()),
        },
        KeyCode::Up | KeyCode::Char('k') => app.view.move_by(-1, len),
        KeyCode::Down | KeyCode::Char('j') => app.view.move_by(1, len),
        KeyCode::PageUp => app.view.move_by(-page, len),
        KeyCode::PageDown => app.view.move_by(page, len),
        KeyCode::Home | KeyCode::Char('g') => app.view.first(),
        KeyCode::End | KeyCode::Char('G') => app.view.last(len),
        _ => {}
    }
    false
}

fn handle_menu_key(app: &mut AppUi, key: &event::KeyEvent, state: &UpDashboardState) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('q') => app.menu_open = false,
        KeyCode::Up | KeyCode::Char('k') => app.menu_index = app.menu_index.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            if app.menu_index + 1 < MENU_ITEMS.len() {
                app.menu_index += 1;
            }
        }
        KeyCode::Enter => {
            let selected = selected_request_text(app, state);
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
                app.menu_open = false;
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
    let len = requests(state).len();
    let pos = Position { x: mouse.column, y: mouse.row };
    let row_index = || {
        app.list_area
            .contains(pos)
            .then(|| app.view.offset + (mouse.row - app.list_area.y) as usize)
    };

    match mouse.kind {
        MouseEventKind::ScrollUp => app.view.move_by(-3, len),
        MouseEventKind::ScrollDown => app.view.move_by(3, len),
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(i) = row_index() {
                app.view.select(i, len);
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some(i) = row_index() {
                app.view.select(i, len);
                if let Some(line) = selected_request_text(app, state) {
                    app.set_toast(copy_line(&line));
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------------------------

fn truncate_to_width(text: &str, max_cols: usize) -> String {
    if text.chars().count() <= max_cols {
        return text.to_string();
    }
    if max_cols == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(max_cols - 1).collect();
    out.push('…');
    out
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms == 0 {
        "<1ms".to_string()
    } else if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

fn format_uptime(d: Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::Green,
        300..=399 => Color::Cyan,
        400..=499 => Color::Yellow,
        500..=599 => Color::Red,
        _ => Color::Gray,
    }
}

fn method_color(method: &str) -> Color {
    match method {
        "GET" | "HEAD" => Color::Cyan,
        "POST" => Color::Green,
        "PUT" | "PATCH" => Color::Yellow,
        "DELETE" => Color::Red,
        _ => Color::Gray,
    }
}

fn request_line(e: &RequestEntry, width: usize, selected: bool) -> Line<'static> {
    const FIXED: usize = 8 + 2 + 7 + 1 + 3 + 2; // time, gap, method, gap, status, gap
    let dur = format_duration(e.duration);
    let path_width = width.saturating_sub(FIXED + dur.chars().count() + 2);
    let path = truncate_to_width(&e.path, path_width);
    let pad = " ".repeat(path_width.saturating_sub(path.chars().count()) + 2);
    let time = DateTime::<Local>::from(e.at).format("%H:%M:%S").to_string();

    let row = if selected {
        Style::default().bg(Color::Rgb(38, 44, 52))
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(time, Style::default().fg(DIM)),
        Span::raw("  "),
        Span::styled(
            format!("{:<7}", e.method),
            Style::default().fg(method_color(&e.method)).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:>3}", e.status),
            Style::default().fg(status_color(e.status)).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(path),
        Span::raw(pad),
        Span::styled(dur, Style::default().fg(DIM)),
    ])
    .style(row)
}

// ---------------------------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------------------------

fn draw(frame: &mut ratatui::Frame, app: &mut AppUi, state: &UpDashboardState) {
    let area = frame.area().inner(Margin { horizontal: 2, vertical: 1 });
    let tls_failures = state.telemetry.tls_handshake_failures();
    let info_height = if tls_failures > 0 { 3 } else { 2 };
    let logs_height = if app.show_logs { LOGS_HEIGHT.min(area.height / 3) } else { 0 };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),           // header
            Constraint::Length(1),           // spacer
            Constraint::Length(info_height), // url + details
            Constraint::Length(1),           // spacer
            Constraint::Min(3),              // requests
            Constraint::Length(logs_height), // logs (optional)
            Constraint::Length(1),           // footer
        ])
        .split(area);

    draw_header(frame, rows[0], app, state);
    draw_info(frame, rows[2], state, tls_failures);
    draw_requests(frame, rows[4], app, state);
    if logs_height > 0 {
        draw_logs(frame, rows[5], state);
    }
    draw_footer(frame, rows[6], app);

    if app.menu_open {
        draw_menu(frame, app);
    }
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &AppUi, state: &UpDashboardState) {
    let left = Line::from(vec![
        Span::styled("◉ CleanServe", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(state.project_name.clone(), Style::default().add_modifier(Modifier::BOLD)),
    ]);
    let right = Line::from(vec![
        Span::styled("● ", Style::default().fg(Color::Green)),
        Span::styled("running", Style::default().fg(Color::Green)),
        Span::styled(format!("  {}", format_uptime(app.started.elapsed())), Style::default().fg(DIM)),
    ]);
    frame.render_widget(Paragraph::new(left), area);
    frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), area);
}

fn draw_info(frame: &mut ratatui::Frame, area: Rect, state: &UpDashboardState, tls_failures: u64) {
    let mut details = vec![format!("PHP {}", state.php_version), state.document_root.clone()];
    if state.framework != "Unknown" {
        details.push(state.framework.clone());
    }
    details.push(format!("hot reload {}", if state.hot_reload { "on" } else { "off" }));

    let mut lines = vec![
        Line::from(vec![
            Span::styled("➜  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("Local   ", Style::default().fg(DIM)),
            Span::styled(
                state.url.clone(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
        ]),
        Line::from(Span::styled(format!("   {}", details.join(" · ")), Style::default().fg(DIM))),
    ];
    if tls_failures > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "   ⚠ {} HTTPS connection(s) rejected: accept the certificate in your browser or run with --http-only",
                tls_failures
            ),
            Style::default().fg(Color::Yellow),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn section_block(title: Line<'static>) -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(DIM))
        .title(title)
}

fn draw_requests(frame: &mut ratatui::Frame, area: Rect, app: &mut AppUi, state: &UpDashboardState) {
    let entries = requests(state);

    let mut title = vec![
        Span::styled(" Requests", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!(" · {} ", state.telemetry.request_count()), Style::default().fg(DIM)),
    ];
    if !app.view.follow && !entries.is_empty() {
        title.push(Span::styled("· paused, End to follow ", Style::default().fg(Color::Yellow)));
    }
    let block = section_block(Line::from(title));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    app.list_area = inner;
    app.view.sync(entries.len(), inner.height as usize);

    if entries.is_empty() {
        let hint = vec![
            Line::from(""),
            Line::from(Span::styled("Waiting for requests…", Style::default().fg(DIM))),
            Line::from(Span::styled(
                format!("Open {} (press o)", state.url),
                Style::default().fg(DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), inner);
        return;
    }

    let width = inner.width as usize;
    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .skip(app.view.offset)
        .take(inner.height as usize)
        .map(|(i, e)| request_line(e, width, !app.view.follow && i == app.view.selected))
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_logs(frame: &mut ratatui::Frame, area: Rect, state: &UpDashboardState) {
    let block = section_block(Line::from(Span::styled(
        " Logs ",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let logs = state.telemetry.snapshot_logs();
    let visible = inner.height as usize;
    let width = inner.width as usize;
    let lines: Vec<Line> = logs
        .iter()
        .skip(logs.len().saturating_sub(visible))
        .map(|l| Line::from(Span::styled(truncate_to_width(l, width), Style::default().fg(DIM))))
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect, app: &AppUi) {
    if let Some((msg, _)) = &app.toast {
        let text = truncate_to_width(msg, area.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(text, Style::default().fg(Color::Green))),
            area,
        );
        return;
    }

    let hints: [(&str, &str); 6] = [
        ("q", "quit"),
        ("↑↓", "scroll"),
        ("c", "copy"),
        ("o", "open"),
        ("l", if app.show_logs { "hide logs" } else { "logs" }),
        ("m", "more"),
    ];
    let mut spans = Vec::new();
    for (key, label) in hints {
        spans.push(Span::styled(key, Style::default().add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {}   ", label), Style::default().fg(DIM)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_menu(frame: &mut ratatui::Frame, app: &AppUi) {
    let area = centered_rect(56, MENU_ITEMS.len() as u16 + 2, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = MENU_ITEMS.iter().map(|label| ListItem::new(format!(" {}", label))).collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT))
                .title(" More · Enter run · Esc close "),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(ACCENT))
        .highlight_symbol("▸");

    let mut list_state = ListState::default();
    list_state.select(Some(app.menu_index));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(height), Constraint::Fill(1)])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Percentage(percent_x), Constraint::Fill(1)])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::SystemTime;

    fn state() -> UpDashboardState {
        UpDashboardState {
            project_name: "demo-app".into(),
            project_root: PathBuf::from("."),
            php_version: "8.4".into(),
            document_root: "public/".into(),
            url: "https://localhost:8080".into(),
            framework: "Unknown".into(),
            hot_reload: true,
            telemetry: Arc::new(RuntimeTelemetry::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    fn add(state: &UpDashboardState, method: &str, path: &str, status: u16, ms: u64) {
        state.telemetry.record_request();
        state.telemetry.push_request(RequestEntry {
            at: SystemTime::now(),
            method: method.into(),
            path: path.into(),
            status,
            duration: Duration::from_millis(ms),
        });
    }

    fn render(app: &mut AppUi, state: &UpDashboardState, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| draw(f, app, state)).unwrap();
        let buf = terminal.backend().buffer().clone();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn empty_state_shows_url_and_waiting_hint() {
        let st = state();
        let mut app = AppUi::new();
        let screen = render(&mut app, &st, 80, 20);
        assert!(screen.contains("CleanServe"), "{screen}");
        assert!(screen.contains("demo-app"), "{screen}");
        assert!(screen.contains("https://localhost:8080"), "{screen}");
        assert!(screen.contains("PHP 8.4 · public/ · hot reload on"), "{screen}");
        assert!(screen.contains("Waiting for requests"), "{screen}");
        assert!(screen.contains("q quit"), "{screen}");
    }

    #[test]
    fn requests_show_method_status_path_and_duration() {
        let st = state();
        add(&st, "GET", "/", 200, 12);
        add(&st, "PUT", "/api/items/1", 404, 3);
        add(&st, "POST", "/login", 500, 1500);
        let mut app = AppUi::new();
        let screen = render(&mut app, &st, 80, 20);
        assert!(screen.contains("Requests · 3"), "{screen}");
        assert!(screen.contains("GET") && screen.contains("200"), "{screen}");
        assert!(screen.contains("/api/items/1") && screen.contains("404"), "{screen}");
        assert!(screen.contains("1.5s"), "{screen}");
        assert!(screen.contains("12ms"), "{screen}");
    }

    #[test]
    fn long_paths_are_truncated_not_wrapped() {
        let st = state();
        add(&st, "GET", &format!("/{}", "x".repeat(200)), 200, 5);
        let mut app = AppUi::new();
        let screen = render(&mut app, &st, 60, 16);
        assert!(screen.contains('…'), "{screen}");
        assert!(screen.lines().all(|l| l.chars().count() <= 60), "{screen}");
    }

    #[test]
    fn tls_warning_and_logs_panel() {
        let st = state();
        st.telemetry.record_tls_handshake_failure();
        st.telemetry.push_log("Hot reload enabled");
        let mut app = AppUi::new();
        app.show_logs = true;
        let screen = render(&mut app, &st, 100, 24);
        assert!(screen.contains("rejected"), "{screen}");
        assert!(screen.contains("Logs"), "{screen}");
        assert!(screen.contains("Hot reload enabled"), "{screen}");
        assert!(screen.contains("hide logs"), "{screen}");
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let st = state();
        add(&st, "GET", "/", 200, 1);
        let mut app = AppUi::new();
        for (w, h) in [(1, 1), (10, 3), (20, 6), (40, 10)] {
            render(&mut app, &st, w, h);
        }
    }

    #[test]
    fn list_follows_tail_until_user_scrolls_up() {
        let mut v = ListView::new();
        v.sync(20, 5);
        assert_eq!((v.selected, v.offset, v.follow), (19, 15, true));

        v.move_by(-1, 20);
        assert!(!v.follow);
        v.sync(25, 5); // new requests arrive: view must not jump
        assert_eq!(v.selected, 18);

        v.last(25);
        v.sync(25, 5);
        assert_eq!((v.selected, v.offset, v.follow), (24, 20, true));

        v.move_by(-100, 25);
        assert_eq!(v.selected, 0);
        v.move_by(100, 25);
        assert!(v.follow && v.selected == 24);
    }

    #[test]
    fn list_view_handles_empty_and_resize() {
        let mut v = ListView::new();
        v.sync(0, 10);
        v.move_by(5, 0);
        assert_eq!(v, ListView::new());
        v.sync(3, 0); // zero-height viewport
        assert_eq!(v.selected, 2);
    }

    #[test]
    fn formatting_helpers() {
        assert_eq!(format_duration(Duration::from_micros(200)), "<1ms");
        assert_eq!(format_duration(Duration::from_millis(42)), "42ms");
        assert_eq!(format_duration(Duration::from_millis(2500)), "2.5s");
        assert_eq!(format_uptime(Duration::from_secs(75)), "01:15");
        assert_eq!(format_uptime(Duration::from_secs(3725)), "1:02:05");
        assert_eq!(truncate_to_width("abcdef", 4), "abc…");
        assert_eq!(truncate_to_width("abc", 4), "abc");
        assert_eq!(truncate_to_width("abc", 0), "");
        assert_eq!(status_color(204), Color::Green);
        assert_eq!(status_color(404), Color::Yellow);
        assert_eq!(status_color(503), Color::Red);
    }
}
