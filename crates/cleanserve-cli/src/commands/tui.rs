//! Terminal UI for CleanServe version management (ratatui).

use super::selfupdate::{install_release, Progress, Release, Stage, CURRENT};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use semver::Version;
use std::time::Duration;

const ACCENT: Color = Color::Cyan;
const OK: Color = Color::Green;
const DIM: Color = Color::DarkGray;

fn frame_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

fn header(f: &mut Frame, area: Rect, subtitle: &str) {
    let line = Line::from(vec![
        Span::styled(" 🌀 CleanServe ", Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(subtitle.to_string(), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("   installed: v{CURRENT}"), Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(line).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(DIM))), area);
}

/// Interactive release picker. Returns the chosen release, or None if cancelled.
pub fn pick_release(releases: &[Release]) -> anyhow::Result<Option<Release>> {
    let mut terminal = ratatui::init();
    let result = pick_loop(&mut terminal, releases);
    ratatui::restore();
    result
}

fn pick_loop(terminal: &mut DefaultTerminal, releases: &[Release]) -> anyhow::Result<Option<Release>> {
    let mut state = ListState::default();
    state.select(Some(releases.iter().position(|r| r.is_current()).unwrap_or(0)));
    let current = Version::parse(CURRENT).ok();

    loop {
        terminal.draw(|f| {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(5), Constraint::Length(1)])
                .split(f.area());
            header(f, rows[0], "Version manager");

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
                .split(rows[1]);

            let items: Vec<ListItem> = releases
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let mut spans = vec![Span::styled(
                        format!("v{:<10}", r.label()),
                        Style::default().add_modifier(Modifier::BOLD),
                    )];
                    if i == 0 && !r.prerelease {
                        spans.push(Span::styled(" latest", Style::default().fg(OK)));
                    }
                    if r.prerelease {
                        spans.push(Span::styled(" pre", Style::default().fg(Color::Yellow)));
                    }
                    if r.is_current() {
                        spans.push(Span::styled(" ● installed", Style::default().fg(ACCENT)));
                    }
                    ListItem::new(Line::from(spans))
                })
                .collect();
            let list = List::new(items)
                .block(frame_block(format!(" Releases ({}) ", releases.len())))
                .highlight_style(Style::default().bg(Color::Rgb(30, 50, 60)).add_modifier(Modifier::BOLD))
                .highlight_symbol("▶ ");
            f.render_stateful_widget(list, cols[0], &mut state);

            let sel = state.selected().and_then(|i| releases.get(i));
            let detail = match sel {
                Some(r) => {
                    let action = match (r.version(), &current) {
                        (Some(v), Some(c)) if v > *c => ("⬆ upgrade", OK),
                        (Some(v), Some(c)) if v < *c => ("⬇ downgrade", Color::Yellow),
                        _ => ("● current version", ACCENT),
                    };
                    let size = r
                        .platform_asset()
                        .map(|a| format!("{:.1} MB", a.size as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "no build for this platform".into());
                    let date = r.published_at.as_deref().and_then(|d| d.get(..10)).unwrap_or("-");
                    let mut lines = vec![
                        Line::from(Span::styled(format!("v{}", r.label()), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
                        Line::from(Span::styled(action.0, Style::default().fg(action.1))),
                        Line::from(""),
                        Line::from(vec![Span::styled("Released  ", Style::default().fg(DIM)), Span::raw(date)]),
                        Line::from(vec![Span::styled("Download  ", Style::default().fg(DIM)), Span::raw(size)]),
                        Line::from(""),
                        Line::from(Span::styled("Release notes", Style::default().add_modifier(Modifier::UNDERLINED))),
                    ];
                    for l in r.body.as_deref().unwrap_or("(no notes)").lines().take(30) {
                        lines.push(Line::from(l.to_string()));
                    }
                    lines
                }
                None => vec![Line::from("No release selected")],
            };
            f.render_widget(
                Paragraph::new(detail).wrap(Wrap { trim: false }).block(frame_block(" Details ".into())),
                cols[1],
            );

            let keys = Line::from(vec![
                Span::styled(" ↑/↓ ", Style::default().fg(Color::Black).bg(DIM)),
                Span::raw(" navigate  "),
                Span::styled(" Enter ", Style::default().fg(Color::Black).bg(OK)),
                Span::raw(" install  "),
                Span::styled(" q ", Style::default().fg(Color::Black).bg(DIM)),
                Span::raw(" quit"),
            ]);
            f.render_widget(Paragraph::new(keys), rows[2]);
        })?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let len = releases.len();
        let cur = state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
            KeyCode::Down | KeyCode::Char('j') => state.select(Some((cur + 1).min(len.saturating_sub(1)))),
            KeyCode::Up | KeyCode::Char('k') => state.select(Some(cur.saturating_sub(1))),
            KeyCode::Home | KeyCode::Char('g') => state.select(Some(0)),
            KeyCode::End | KeyCode::Char('G') => state.select(Some(len.saturating_sub(1))),
            KeyCode::Enter => return Ok(releases.get(cur).cloned()),
            _ => {}
        }
    }
}

/// Run the install with a live progress screen.
pub async fn install_screen(release: &Release) -> anyhow::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let rel = release.clone();
    let handle = tokio::spawn(async move { install_release(&rel, tx).await });

    let mut terminal = ratatui::init();
    let mut stage = Stage::Backup;
    let mut dl = (0u64, 0u64);
    let mut finished_at: Option<std::time::Instant> = None;

    let outcome = loop {
        while let Ok(p) = rx.try_recv() {
            match p {
                Progress::Stage(s) => stage = s,
                Progress::Download { done, total } => dl = (done, total),
            }
        }
        let done = handle.is_finished();
        if done && finished_at.is_none() {
            finished_at = Some(std::time::Instant::now());
        }
        let failed = done && finished_at.is_some() && stage != Stage::Done;

        terminal.draw(|f| {
            let area = f.area();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(8), Constraint::Length(1)])
                .split(area);
            header(f, rows[0], &format!("Installing v{}", release.label()));

            let mut lines = vec![Line::from("")];
            for s in Stage::ALL {
                let (icon, style) = if stage == Stage::Done || s < stage {
                    ("✔", Style::default().fg(OK))
                } else if s == stage && failed {
                    ("✘", Style::default().fg(Color::Red))
                } else if s == stage {
                    ("●", Style::default().fg(ACCENT))
                } else {
                    ("○", Style::default().fg(DIM))
                };
                lines.push(Line::from(vec![Span::raw("  "), Span::styled(icon, style), Span::raw(" "), Span::styled(s.label(), style)]));
            }
            let inner = frame_block(" Progress ".into());
            let body = inner.inner(rows[1]);
            f.render_widget(inner, rows[1]);
            f.render_widget(Paragraph::new(lines), body);

            let g_area = Rect { x: body.x + 2, y: body.y + 8, width: body.width.saturating_sub(4), height: 1 };
            if g_area.y < body.y + body.height {
                let ratio = if dl.1 > 0 { (dl.0 as f64 / dl.1 as f64).clamp(0.0, 1.0) } else { 0.0 };
                f.render_widget(
                    Gauge::default()
                        .gauge_style(Style::default().fg(ACCENT).bg(Color::Rgb(30, 40, 45)))
                        .ratio(if stage > Stage::Download { 1.0 } else { ratio })
                        .label(format!("{:.1} / {:.1} MB", dl.0 as f64 / 1_048_576.0, dl.1 as f64 / 1_048_576.0)),
                    g_area,
                );
            }
            let msg = if stage == Stage::Done {
                Span::styled(" ✔ Updated — restart any running `cleanserve up` ", Style::default().fg(OK))
            } else if failed {
                Span::styled(" ✘ Failed — see error below ", Style::default().fg(Color::Red))
            } else {
                Span::styled(" Working… ", Style::default().fg(DIM))
            };
            f.render_widget(Paragraph::new(Line::from(msg)), rows[2]);
        })?;

        if let Some(t) = finished_at {
            if t.elapsed() > Duration::from_millis(900) {
                break handle.await;
            }
        }
        if event::poll(Duration::from_millis(60))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press
                    && k.code == KeyCode::Char('c')
                    && k.modifiers.contains(KeyModifiers::CONTROL)
                {
                    handle.abort();
                    ratatui::restore();
                    anyhow::bail!("Cancelled");
                }
            }
        }
    };
    ratatui::restore();
    outcome??;
    Ok(())
}
