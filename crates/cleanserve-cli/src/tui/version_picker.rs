//! Terminal UI for CleanServe version management (ratatui).

use cleanserve_core::auto_updater::{GitHubRelease, UpdateChecker};
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

fn header(f: &mut Frame, area: Rect, subtitle: &str, current_label: &str) {
    let line = Line::from(vec![
        Span::styled(" 🌀 CleanServe ", Style::default().fg(Color::Black).bg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(subtitle.to_string(), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("   installed: v{current_label}"), Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(line).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(DIM))), area);
}

/// Interactive release picker. Returns the chosen release, or None if cancelled.
pub fn pick_release(releases: &[GitHubRelease], current: &str) -> anyhow::Result<Option<GitHubRelease>> {
    let mut terminal = ratatui::init();
    let result = pick_loop(&mut terminal, releases, current);
    ratatui::restore();
    result
}

fn pick_loop(terminal: &mut DefaultTerminal, releases: &[GitHubRelease], current_label: &str) -> anyhow::Result<Option<GitHubRelease>> {
    let mut state = ListState::default();
    state.select(Some(releases.iter().position(|r| r.tag_name.trim_start_matches('v') == current_label).unwrap_or(0)));
    let current = Version::parse(current_label).ok();

    loop {
        terminal.draw(|f| {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(5), Constraint::Length(1)])
                .split(f.area());
            header(f, rows[0], "Version manager", current_label);

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
                .split(rows[1]);

            let items: Vec<ListItem> = releases
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let mut spans = vec![Span::styled(
                        format!("v{:<10}", r.tag_name.trim_start_matches('v')),
                        Style::default().add_modifier(Modifier::BOLD),
                    )];
                    if i == 0 && !r.prerelease {
                        spans.push(Span::styled(" latest", Style::default().fg(OK)));
                    }
                    if r.prerelease {
                        spans.push(Span::styled(" pre", Style::default().fg(Color::Yellow)));
                    }
                    if (r.tag_name.trim_start_matches('v') == current_label) {
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
                    let action = match (r.semver(), &current) {
                        (Some(v), Some(c)) if v > *c => ("⬆ upgrade", OK),
                        (Some(v), Some(c)) if v < *c => ("⬇ downgrade", Color::Yellow),
                        _ => ("● current version", ACCENT),
                    };
                    let asset = UpdateChecker::asset_name().ok();
                    let size = r
                        .assets
                        .iter()
                        .find(|a| Some(&a.name) == asset.as_ref())
                        .map(|a| format!("{:.1} MB", a.size as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "no build for this platform".into());
                    let date = r.published_at.as_deref().and_then(|d| d.get(..10)).unwrap_or("-");
                    let mut lines = vec![
                        Line::from(Span::styled(format!("v{}", r.tag_name.trim_start_matches('v')), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
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

