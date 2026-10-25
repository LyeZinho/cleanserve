use cleanserve_core::RuntimeTelemetry;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::DefaultTerminal;
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;

pub struct UpDashboardState {
    pub project_name: String,
    pub php_version: String,
    pub document_root: String,
    pub url: String,
    pub framework: String,
    pub hot_reload: bool,
    pub telemetry: Arc<RuntimeTelemetry>,
}

pub fn run_up_dashboard(state: UpDashboardState) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, &state);
    ratatui::restore();
    disable_raw_mode()?;
    stdout.execute(LeaveAlternateScreen)?;
    result
}

fn run_loop(terminal: &mut DefaultTerminal, state: &UpDashboardState) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, state))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame, state: &UpDashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" CleanServe ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" — {} ", state.project_name)),
        Span::styled(state.url.clone(), Style::default().fg(Color::Green)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Server"));
    frame.render_widget(header, chunks[0]);

    let info = Paragraph::new(vec![
        Line::from(format!("Framework: {}", state.framework)),
        Line::from(format!("PHP: {}  |  Root: {}", state.php_version, state.document_root)),
        Line::from(format!(
            "Hot reload: {}  |  Requests: {}",
            if state.hot_reload { "on" } else { "off" },
            state.telemetry.request_count()
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(info, Rect {
        x: chunks[1].x,
        y: chunks[1].y,
        width: chunks[1].width,
        height: chunks[1].height / 2,
    });

    let logs = state.telemetry.snapshot_logs();
    let items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(32)
        .map(|l| ListItem::new(l.as_str()))
        .collect();
    let log_list = List::new(items).block(Block::default().borders(Borders::ALL).title("Requests"));
    frame.render_widget(
        log_list,
        Rect {
            x: chunks[1].x,
            y: chunks[1].y + chunks[1].height / 2,
            width: chunks[1].width,
            height: chunks[1].height - chunks[1].height / 2,
        },
    );

    let footer = Paragraph::new(" q or Ctrl+C — stop server ")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}
