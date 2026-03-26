use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::server::metrics::{MetricsSnapshot, SERVER_METRICS};
use crate::ws::session::SESSION_MANAGER;

// Renegade BBS color theme
const BG: Color = Color::Blue;
const BORDER: Color = Color::Cyan;
const LABEL: Color = Color::Cyan;
const VALUE: Color = Color::White;
const EMPHASIS: Color = Color::Yellow;

/// TUI application state.
struct TuiApp {
    port: u16,
    db_path: String,
    active_scroll: usize,
    recent_scroll: usize,
    quit_confirm: bool,
    tick_counter: u64,
}

impl TuiApp {
    fn new(port: u16, db_path: String) -> Self {
        Self {
            port,
            db_path,
            active_scroll: 0,
            recent_scroll: 0,
            quit_confirm: false,
            tick_counter: 0,
        }
    }
}

/// Public entry point: run the TUI event loop. Blocks until quit.
/// Returns Ok(()) on clean exit, Err on terminal failure.
pub fn run_tui(port: u16, db_path: String) -> io::Result<()> {
    // Install panic hook to restore terminal on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(port, db_path);
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();
    let mut last_metrics_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.quit_confirm {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                break;
                            }
                            _ => {
                                app.quit_confirm = false;
                            }
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                app.quit_confirm = true;
                            }
                            KeyCode::Up => {
                                app.active_scroll = app.active_scroll.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                app.active_scroll = app.active_scroll.saturating_add(1);
                            }
                            KeyCode::PageUp => {
                                app.recent_scroll = app.recent_scroll.saturating_sub(1);
                            }
                            KeyCode::PageDown => {
                                app.recent_scroll = app.recent_scroll.saturating_add(1);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick_counter += 1;
            last_tick = Instant::now();
        }

        // Call metrics tick() every 60 seconds
        if last_metrics_tick.elapsed() >= Duration::from_secs(60) {
            SERVER_METRICS.tick();
            last_metrics_tick = Instant::now();
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw(f: &mut ratatui::Frame, app: &TuiApp) {
    let size = f.area();

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(BG));
    f.render_widget(bg_block, size);

    let snap = SERVER_METRICS.snapshot();

    // Gather session info from SESSION_MANAGER + metrics cache
    let active_sessions = gather_active_sessions(&snap);
    let recent_sessions = gather_recent_sessions(&snap);

    // Main layout: title (1), stats row (6), active sessions (variable), recent (7), status (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Length(8), // stats panels
            Constraint::Min(5),    // active sessions
            Constraint::Length(9), // recent sessions
            Constraint::Length(3), // status bar
        ])
        .split(size);

    draw_title_bar(f, chunks[0]);
    draw_stats_row(f, chunks[1], &snap, app);
    draw_active_sessions(f, chunks[2], &active_sessions, app.active_scroll);
    draw_recent_sessions(f, chunks[3], &recent_sessions, app.recent_scroll);
    draw_status_bar(f, chunks[4], app);
}

fn draw_title_bar(f: &mut ratatui::Frame, area: Rect) {
    let now = chrono::Local::now();
    let time_str = now.format("%l:%M %p").to_string();
    let date_str = now.format("%Y-%m-%d").to_string();
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("WebRPG v{version} Server");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Server Status ",
            Style::default()
                .fg(EMPHASIS)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 20 {
        return;
    }

    let time_span = Span::styled(
        time_str.trim().to_string(),
        Style::default().fg(VALUE).bg(BG),
    );
    let title_span = Span::styled(
        title,
        Style::default()
            .fg(VALUE)
            .bg(BG)
            .add_modifier(Modifier::BOLD),
    );
    let date_span = Span::styled(date_str, Style::default().fg(VALUE).bg(BG));

    // Center the title, put time left, date right
    let title_width = title_span.width() as u16;
    let time_width = time_span.width() as u16;
    let date_width = date_span.width() as u16;

    let time_area = Rect::new(inner.x + 1, inner.y, time_width, 1);
    let title_x = inner
        .x
        .saturating_add(inner.width.saturating_sub(title_width) / 2);
    let title_area = Rect::new(title_x, inner.y, title_width, 1);
    let date_x = inner
        .x
        .saturating_add(inner.width.saturating_sub(date_width + 1));
    let date_area = Rect::new(date_x, inner.y, date_width, 1);

    f.render_widget(Paragraph::new(Line::from(time_span)), time_area);
    f.render_widget(Paragraph::new(Line::from(title_span)), title_area);
    f.render_widget(Paragraph::new(Line::from(date_span)), date_area);
}

fn draw_stats_row(f: &mut ratatui::Frame, area: Rect, snap: &MetricsSnapshot, app: &TuiApp) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    draw_live_stats(f, cols[0], snap);
    draw_averages(f, cols[1], snap);
    draw_server_info(f, cols[2], snap, app);
}

fn draw_live_stats(f: &mut ratatui::Frame, area: Rect, snap: &MetricsSnapshot) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Live Stats ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let active_count = SESSION_MANAGER.sessions.len();
    let ws_conns = snap.ws_connections;
    let peak = snap.peak_connections_24h;

    // Count total users across all sessions
    let users_online: usize = SESSION_MANAGER
        .sessions
        .iter()
        .map(|s| s.clients.len())
        .sum();

    let lines = vec![
        stat_line("Users Online", &users_online.to_string()),
        stat_line("Peak (24h)", &peak.to_string()),
        stat_line("Sessions Active", &active_count.to_string()),
        stat_line("WebSocket Conns", &ws_conns.to_string()),
    ];

    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), inner);
}

fn draw_averages(f: &mut ratatui::Frame, area: Rect, snap: &MetricsSnapshot) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Averages ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let lines = vec![
        stat_line("Req/min (1m)", &format!("{:.1}", snap.ewma_http_1m)),
        stat_line("Req/min (5m)", &format!("{:.1}", snap.ewma_http_5m)),
        stat_line("Req/min (15m)", &format!("{:.1}", snap.ewma_http_15m)),
        stat_line("WS Msgs/min", &format!("{:.1}", snap.ewma_ws_1m)),
    ];

    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), inner);
}

fn draw_server_info(f: &mut ratatui::Frame, area: Rect, snap: &MetricsSnapshot, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Server Info ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let uptime = format_uptime(snap.uptime_secs);
    let version = env!("CARGO_PKG_VERSION");
    let db_size = get_db_size(&app.db_path);

    let lines = vec![
        stat_line("Uptime", &uptime),
        stat_line("Version", version),
        stat_line("Port", &app.port.to_string()),
        stat_line("DB Size", &db_size),
    ];

    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), inner);
}

struct SessionRow {
    name: String,
    gm: String,
    players: usize,
    time: String,
}

fn gather_active_sessions(snap: &MetricsSnapshot) -> Vec<SessionRow> {
    snap.recent_sessions
        .iter()
        .filter(|s| s.active)
        .map(|s| SessionRow {
            name: s.name.clone(),
            gm: s.gm_username.clone(),
            players: s.player_count,
            time: s.created_at.clone(),
        })
        .collect()
}

fn gather_recent_sessions(snap: &MetricsSnapshot) -> Vec<SessionRow> {
    let mut recent: Vec<_> = snap.recent_sessions.iter().filter(|s| !s.active).collect();
    recent.sort_by(|a, b| b.last_active.cmp(&a.last_active));
    recent.truncate(5);
    recent
        .into_iter()
        .map(|s| SessionRow {
            name: s.name.clone(),
            gm: s.gm_username.clone(),
            players: s.player_count,
            time: format!("{:.0}m ago", s.last_active.elapsed().as_secs() / 60),
        })
        .collect()
}

fn draw_active_sessions(
    f: &mut ratatui::Frame,
    area: Rect,
    sessions: &[SessionRow],
    scroll: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Active Sessions ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let header = Row::new(vec![
        Cell::from("#").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("Session Name").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("GM").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("Players").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("Created").style(Style::default().fg(LABEL).bg(BG)),
    ])
    .height(1)
    .style(
        Style::default()
            .fg(LABEL)
            .bg(BG)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = sessions
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(i, s)| {
            Row::new(vec![
                Cell::from(format!("{}", i + 1)).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.name.clone()).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.gm.clone()).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.players.to_string()).style(Style::default().fg(EMPHASIS).bg(BG)),
                Cell::from(s.time.clone()).style(Style::default().fg(VALUE).bg(BG)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Length(8),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

fn draw_recent_sessions(
    f: &mut ratatui::Frame,
    area: Rect,
    sessions: &[SessionRow],
    scroll: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Recent Sessions (last 5) ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let header = Row::new(vec![
        Cell::from("#").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("Session Name").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("GM").style(Style::default().fg(LABEL).bg(BG)),
        Cell::from("Last Active").style(Style::default().fg(LABEL).bg(BG)),
    ])
    .height(1)
    .style(
        Style::default()
            .fg(LABEL)
            .bg(BG)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = sessions
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(i, s)| {
            Row::new(vec![
                Cell::from(format!("{}", i + 1)).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.name.clone()).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.gm.clone()).style(Style::default().fg(VALUE).bg(BG)),
                Cell::from(s.time.clone()).style(Style::default().fg(VALUE).bg(BG)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut ratatui::Frame, area: Rect, app: &TuiApp) {
    let msg = if app.quit_confirm {
        Span::styled(
            "  Quit server? (y/N)  ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "  Server running...  [q] Quit  [Up/Down] Scroll active  [PgUp/PgDn] Scroll recent",
            Style::default().fg(VALUE).bg(BG),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER).bg(BG))
        .title(Span::styled(
            " Status ",
            Style::default().fg(EMPHASIS).bg(BG),
        ))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(Line::from(msg)).style(Style::default().bg(BG)),
        inner,
    );
}

// Helper: format a stat line with label and value
fn stat_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label:<18}"), Style::default().fg(LABEL).bg(BG)),
        Span::styled(value.to_string(), Style::default().fg(VALUE).bg(BG)),
    ])
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}")
    } else {
        format!("{hours}:{minutes:02}")
    }
}

fn get_db_size(db_path: &str) -> String {
    match std::fs::metadata(db_path) {
        Ok(meta) => {
            let bytes = meta.len();
            if bytes >= 1_073_741_824 {
                format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
            } else if bytes >= 1_048_576 {
                format!("{:.1} MB", bytes as f64 / 1_048_576.0)
            } else if bytes >= 1024 {
                format!("{:.1} KB", bytes as f64 / 1024.0)
            } else {
                format!("{bytes} B")
            }
        }
        Err(_) => "N/A".to_string(),
    }
}
