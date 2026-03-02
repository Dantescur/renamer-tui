use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, AppMode, Focus};

// ── Palette ──────────────────────────────────────────────────────────────────

const BG: Color = Color::Rgb(18, 18, 24);
const SURFACE: Color = Color::Rgb(28, 28, 36);
const BORDER_INACTIVE: Color = Color::Rgb(55, 55, 70);
const BORDER_ACTIVE: Color = Color::Rgb(100, 149, 237);
const ACCENT: Color = Color::Rgb(100, 149, 237);
const TEXT: Color = Color::Rgb(220, 220, 230);
const TEXT_DIM: Color = Color::Rgb(120, 120, 140);
const GREEN: Color = Color::Rgb(80, 200, 120);
const YELLOW: Color = Color::Rgb(255, 200, 80);
const RED: Color = Color::Rgb(220, 80, 80);

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    frame.render_widget(ratatui::widgets::Block::default().bg(BG), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Length(3), // path bar
            Constraint::Length(1), // column headers
            Constraint::Min(6),    // file lists
            Constraint::Length(6), // log panel
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(frame, chunks[0]);
    render_path_bar(app, frame, chunks[1]);
    render_column_headers(frame, chunks[2]);
    render_file_lists(app, frame, chunks[3]);
    render_log(app, frame, chunks[4]);
    render_help(app, frame, chunks[5]);

    if app.mode == AppMode::ConfirmDialog {
        render_confirm_dialog(app, frame, area);
    }
    if app.mode == AppMode::Done {
        render_done_overlay(frame, area);
    }
}

// ── Title bar ────────────────────────────────────────────────────────────────

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(" 🎬  Series Renamer")
        .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left);
    frame.render_widget(title, area);
}

// ── Path bar ─────────────────────────────────────────────────────────────────

fn render_path_bar(app: &App, frame: &mut Frame, area: Rect) {
    let active = app.focus == Focus::PathBar && app.mode == AppMode::Normal;
    let border_color = if active {
        BORDER_ACTIVE
    } else {
        BORDER_INACTIVE
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(16)])
        .split(area);

    // Input field
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" 📂 Folder ", Style::default().fg(TEXT_DIM)))
        .bg(SURFACE);

    let input_value = app.path_input.value();
    let content: Line = if input_value.is_empty() {
        Line::from(Span::styled(
            "Type a path, or Ctrl+O to browse…",
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
        ))
    } else {
        Line::from(Span::styled(input_value, Style::default().fg(TEXT)))
    };

    let input_para = Paragraph::new(content).block(input_block);
    frame.render_widget(input_para, cols[0]);

    // Cursor position
    if active {
        let inner = cols[0];
        let cursor_x = inner.x + 1 + app.path_input.visual_cursor() as u16;
        let cursor_y = inner.y + 1;
        if cursor_x < inner.x + inner.width.saturating_sub(1) {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Browse button
    let btn_style = if active {
        Style::default()
            .fg(BG)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT_DIM).bg(SURFACE)
    };
    let browse_btn = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("Ctrl+O", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Browse"),
    ]))
    .style(btn_style)
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .bg(SURFACE),
    );
    frame.render_widget(browse_btn, cols[1]);
}

// ── Column headers ────────────────────────────────────────────────────────────

fn render_column_headers(frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left = Paragraph::new("  Original filename")
        .style(Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD));
    let right = Paragraph::new("  Preview (new name)")
        .style(Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD));

    frame.render_widget(left, cols[0]);
    frame.render_widget(right, cols[1]);
}

// ── File lists ────────────────────────────────────────────────────────────────

fn render_file_lists(app: &mut App, frame: &mut Frame, area: Rect) {
    let active_list = app.focus == Focus::FileList && app.mode == AppMode::Normal;
    let border_color = if active_list {
        BORDER_ACTIVE
    } else {
        BORDER_INACTIVE
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let original_items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let selected = i == app.selected && active_list;

            let prefix = if e.skipped { "⏭️" } else { " " };
            let display_name = format!("{}{}", prefix, e.original);
            let style = if e.skipped {
                if selected {
                    Style::default()
                        .fg(BG)
                        .bg(TEXT_DIM)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                        .fg(TEXT_DIM)
                        .add_modifier(Modifier::CROSSED_OUT)
                }
            } else if selected {
                Style::default()
                    .fg(BG)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };

            ListItem::new(display_name).style(style)
        })
        .collect();

    let preview_items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let selected = i == app.selected && active_list;

            let prefix = if e.skipped { "⏭️" } else { "  " };

            let (label, base_style) = match (&e.new_name, e.already_done) {
                (_, true) => (
                    format!(" {} {} ✓", prefix, e.original),
                    if selected {
                        Style::default().fg(BG).bg(GREEN)
                    } else {
                        Style::default().fg(GREEN)
                    },
                ),
                (Some(name), false) => (
                    format!(" {} {}", prefix, name),
                    if selected {
                        Style::default()
                            .fg(BG)
                            .bg(ACCENT)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(ACCENT)
                    },
                ),
                (None, _) => (
                    format!("{} ⚠️  Cannot rename", prefix),
                    if selected {
                        Style::default().fg(BG).bg(YELLOW)
                    } else {
                        Style::default().fg(YELLOW)
                    },
                ),
            };

            let style = if e.skipped {
                base_style.fg(TEXT_DIM).add_modifier(Modifier::CROSSED_OUT)
            } else {
                base_style
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .bg(SURFACE);

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .bg(SURFACE);

    let left_list = List::new(original_items).block(left_block);
    let right_list = List::new(preview_items).block(right_block);

    let mut left_state = ListState::default();
    let mut right_state = ListState::default();
    if !app.entries.is_empty() {
        left_state.select(Some(app.selected));
        right_state.select(Some(app.selected));
    }

    frame.render_stateful_widget(left_list, cols[0], &mut left_state);
    frame.render_stateful_widget(right_list, cols[1], &mut right_state);

    // Empty state hint
    if app.entries.is_empty() {
        let hint = Paragraph::new(
            "No files scanned yet.\nEnter a path above and press Enter, or Ctrl+O to browse.",
        )
        .style(Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let hint_area = Rect {
            x: area.x + 1,
            y: area.y + area.height / 2,
            width: area.width.saturating_sub(2),
            height: 2,
        };
        frame.render_widget(hint, hint_area);
    }
}

// ── Log panel ────────────────────────────────────────────────────────────────

pub fn render_log(app: &mut App, frame: &mut Frame, area: Rect) {
    let active = app.focus == Focus::Log && app.mode == AppMode::Normal;
    let border_color = if active {
        BORDER_ACTIVE
    } else {
        BORDER_INACTIVE
    };

    // Viewport height = panel height minus top+bottom borders
    let viewport_height = area.height.saturating_sub(2);
    app.log_viewport_height = viewport_height;

    let total_lines = app.log.len() as u16;
    let scroll = app.log_scroll;

    // Build title with optional scroll indicator
    let title_text = if total_lines > viewport_height {
        let bottom = (scroll + viewport_height).min(total_lines);
        format!(" Log  {}/{}  ", bottom, total_lines)
    } else {
        " Log ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title_text,
            Style::default().fg(if active { ACCENT } else { TEXT_DIM }),
        ))
        .bg(SURFACE);

    let lines: Vec<Line> = app
        .log
        .iter()
        .map(|l| {
            let color = if l.starts_with("✅") {
                GREEN
            } else if l.starts_with("❌") {
                RED
            } else if l.starts_with("⚠") {
                YELLOW
            } else {
                TEXT_DIM
            };
            Line::from(Span::styled(format!(" {}", l), Style::default().fg(color)))
        })
        .collect();

    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(para, area);

    // Scrollbar — only shown when content overflows
    if total_lines > viewport_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state =
            ScrollbarState::new(total_lines.saturating_sub(viewport_height) as usize)
                .position(scroll as usize);

        // Inset the scrollbar inside the border on the right edge
        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y + 1,
            width: 1,
            height: viewport_height,
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

// ── Help bar ─────────────────────────────────────────────────────────────────

fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let hints: &[(&str, &str)] = match (&app.mode, &app.focus) {
        (AppMode::Normal, Focus::PathBar) => &[
            ("Enter", "Scan"),
            ("Ctrl+O", "Browse"),
            ("Tab", "→ Files"),
            ("Ctrl+C", "Quit"),
        ],
        (AppMode::Normal, Focus::FileList) => &[
            ("↑↓ / j k", "Navigate"),
            ("Space", "Skip/Include"),
            ("Enter / r", "Rename"),
            ("Tab", "→ Log"),
            ("Tab", "→ Path"),
            ("q", "Quit"),
        ],
        (AppMode::Normal, Focus::Log) => &[
            ("↑↓ / j k", "Scroll"),
            ("PgUp/PgDn", "Page"),
            ("g / G", "Top/Bottom"),
            ("Tab / Esc", "→ Path"),
            ("q", "Quit"),
        ],
        (AppMode::ConfirmDialog, _) => &[("y / Enter", "Confirm"), ("n / Esc", "Cancel")],
        (AppMode::Done, _) => &[("Enter / Esc", "Back")],
    };

    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(k, d)| {
            vec![
                Span::styled(
                    format!("[{}]", k),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {}   ", d), Style::default().fg(TEXT_DIM)),
            ]
        })
        .collect();

    let para = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    frame.render_widget(para, area);
}

// ── Confirm dialog ────────────────────────────────────────────────────────────

fn render_confirm_dialog(app: &App, frame: &mut Frame, area: Rect) {
    let renameable = app
        .entries
        .iter()
        .filter(|e| e.new_name.is_some() && !e.already_done && !e.skipped)
        .count();

    let skipped = app.entries.iter().filter(|e| e.skipped).count();

    let text = if skipped > 0 {
        format!(
            "\n  {} file(s) will be renamed ({} skipped).\n\n  [y / Enter] Confirm    [n / Esc] Cancel",
            renameable, skipped
        )
    } else {
        format!(
            "\n  {} file(s) will be renamed.\n\n  [y / Enter] Confirm    [n / Esc] Cancel",
            renameable
        )
    };

    let w = 54u16;
    let h = 7u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let dialog_area = Rect::new(x, y, w, h);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(
            " Confirm Rename ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .bg(SURFACE);

    let para = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(TEXT))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, dialog_area);
}

// ── Done overlay ──────────────────────────────────────────────────────────────

fn render_done_overlay(frame: &mut Frame, area: Rect) {
    let w = 38u16;
    let h = 5u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let ov = Rect::new(x, y, w, h);

    frame.render_widget(Clear, ov);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(GREEN))
        .bg(SURFACE);

    let para = Paragraph::new("\n  ✅  Rename complete!\n  [Enter / Esc] to continue")
        .block(block)
        .style(Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    frame.render_widget(para, ov);
}
