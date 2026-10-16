//! What floats above the page: context menus, dialogs and toasts.

use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Clear, Paragraph, Wrap};

use super::{display_width, ellipsize, pill_width, tail, PillStyle, Ui, ICON_WIDTH};
use crate::tui::app::{detect_link, Action, Dialog, Field, Hit, MenuItem};
use crate::tui::paint::Icon;

const MENU_MIN_WIDTH: u16 = 26;
/// Icon, gaps and the trailing marker around a menu label.
const MENU_CHROME: u16 = 12;
const DIALOG_WIDTH: u16 = 66;
const TOAST_MAX_WIDTH: u16 = 60;
const BADGE_ROOM: u16 = 18;

/// A raised, rounded box; returns its inner area.
fn panel(ui: &mut Ui, area: Rect) -> Rect {
    let t = ui.theme();
    ui.frame.render_widget(Clear, area);
    ui.rounded(area, t.raised, t.bg);
    Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2))
}

fn menu_width(items: &[MenuItem]) -> u16 {
    let widest = items.iter().map(|item| display_width(&item.label)).max().unwrap_or(0) as u16;
    (widest + MENU_CHROME).max(MENU_MIN_WIDTH)
}

/// Draws one menu level and returns the row rectangles of its items.
fn menu_level(ui: &mut Ui, screen: Rect, anchor: Position, items: &[MenuItem], open: Option<usize>) -> Vec<Rect> {
    let t = ui.theme();
    let width = menu_width(items).min(screen.width);
    let height = (items.len() as u16 + 2).min(screen.height);
    let x = anchor.x.min(screen.right().saturating_sub(width));
    let y = anchor.y.min(screen.bottom().saturating_sub(height));
    let inner = panel(ui, Rect::new(x, y, width, height));
    let mouse = ui.app.mouse;

    let mut rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate().take(inner.height as usize) {
        let row = Rect::new(inner.x, inner.y + index as u16, inner.width, 1);
        let hovered = mouse.is_some_and(|m| row.contains(m)) || open == Some(index);
        let bg = if hovered { t.hover } else { t.raised };
        if hovered {
            ui.rounded(row, bg, t.raised);
        }
        let fg = if item.danger { t.danger } else { t.text };
        let icon_color = if item.danger { t.danger } else { t.muted };
        if let Some(icon) = item.icon {
            ui.glyph(row.x + 1, row.y, icon, icon_color, bg);
        }
        let label_x = row.x + ICON_WIDTH + 3;
        let label = ellipsize(&item.label, row.width.saturating_sub(MENU_CHROME - 2) as usize);
        ui.text(label_x, row.y, row.right().saturating_sub(label_x + ICON_WIDTH + 2), &label, Style::new().fg(fg).bg(bg));
        let marker = if !item.submenu.is_empty() {
            Some((Icon::Forward, t.muted))
        } else if item.checked {
            Some((Icon::Check, t.accent))
        } else {
            None
        };
        if let Some((icon, color)) = marker {
            ui.glyph(row.right() - ICON_WIDTH - 1, row.y, icon, color, bg);
        }
        let action = item.action.clone().unwrap_or(Action::OpenSubmenu(index));
        ui.hits.add(Hit { area: row, click: Some(action), modal: true, ..Hit::default() });
        rows.push(row);
    }
    rows
}

pub fn menu(ui: &mut Ui, screen: Rect) {
    let Some(menu) = &ui.app.menu else { return };
    ui.hits.block(screen, Some(Action::CloseMenu));
    let rows = menu_level(ui, screen, menu.anchor, &menu.items, menu.open_submenu);
    let Some(open) = menu.open_submenu else { return };
    let (Some(item), Some(row)) = (menu.items.get(open), rows.get(open)) else { return };
    if !item.submenu.is_empty() {
        let anchor = Position::new(row.right() + 2, row.y.saturating_sub(1));
        menu_level(ui, screen, anchor, &item.submenu, None);
    }
}

struct DialogText<'a> {
    title: &'a str,
    body: Option<&'a str>,
    /// The text field's content, for dialogs that have one.
    value: Option<&'a str>,
    placeholder: &'a str,
    error: Option<&'a str>,
    confirm: &'a str,
    busy: bool,
    danger: bool,
}

pub fn dialog(ui: &mut Ui, screen: Rect) {
    let app = ui.app;
    let Some(dialog) = &app.dialog else { return };
    let t = ui.theme();
    let dict = &app.dict;
    ui.hits.block(screen, None);

    let text = match dialog {
        Dialog::Import { link, busy, error } => DialogText {
            title: &dict.importer.title,
            body: Some(&dict.importer.body),
            value: Some(link),
            placeholder: &dict.importer.placeholder,
            error: error.as_deref(),
            confirm: if *busy { &dict.importer.working } else { &dict.importer.submit },
            busy: *busy,
            danger: false,
        },
        Dialog::Rename { name, .. } => DialogText {
            title: &dict.playlist.rename,
            body: None,
            value: Some(name),
            placeholder: "",
            error: None,
            confirm: &dict.playlist.rename,
            busy: false,
            danger: false,
        },
        Dialog::Confirm { title, body, confirm, .. } => DialogText {
            title,
            body: Some(body),
            value: None,
            placeholder: "",
            error: None,
            confirm,
            busy: false,
            danger: true,
        },
    };

    let width = DIALOG_WIDTH.min(screen.width.saturating_sub(4));
    let text_width = width.saturating_sub(8).max(1);
    let body_rows = text.body.map_or(0, |b| display_width(b) as u16 / text_width + 2);
    let input_rows = if text.value.is_some() { 4 } else { 0 };
    let error_rows = if text.error.is_some() { 2 } else { 0 };
    let height = (7 + body_rows + input_rows + error_rows).min(screen.height);
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let inner = panel(ui, area);
    let content = Rect::new(inner.x + 2, inner.y + 1, inner.width.saturating_sub(4), inner.height.saturating_sub(1));
    let mut y = content.y;
    let title_style = Style::new().fg(t.text).bg(t.raised).add_modifier(Modifier::BOLD);
    ui.text(content.x, y, content.width, text.title, title_style);
    y += 2;

    if let Some(body) = text.body {
        let style = Style::new().fg(t.muted).bg(t.raised);
        let paragraph = Paragraph::new(body.to_owned()).style(style).wrap(Wrap { trim: true });
        ui.frame.render_widget(paragraph, Rect::new(content.x, y, content.width, body_rows.saturating_sub(1)));
        y += body_rows;
    }

    if let Some(value) = text.value {
        let focused = app.focus == Some(Field::Dialog);
        let fill = if focused { t.active } else { t.surface };
        let input = Rect::new(content.x, y, content.width, 3);
        ui.rounded(input, fill, t.raised);
        let text_y = y + 1;
        let detected = matches!(dialog, Dialog::Import { .. }).then(|| detect_link(value.trim())).flatten();
        let badge_room = if detected.is_some() { BADGE_ROOM } else { 0 };
        let field_width = input.width.saturating_sub(4 + badge_room);
        if value.is_empty() && !focused {
            ui.text(input.x + 2, text_y, field_width, text.placeholder, Style::new().fg(t.faint).bg(fill));
        } else {
            let shown = tail(value, field_width.saturating_sub(1) as usize);
            let used = ui.text(input.x + 2, text_y, field_width, &shown, Style::new().fg(t.text).bg(fill));
            if focused {
                ui.text(input.x + 2 + used, text_y, 1, "▏", Style::new().fg(t.accent).bg(fill));
            }
        }
        if let Some(provider) = detected {
            ui.badge(input.right().saturating_sub(badge_room + 1), text_y, badge_room, provider, fill);
        }
        ui.hits.add(Hit { area: input, field: Some(Field::Dialog), modal: true, ..Hit::default() });
        y += input_rows;
    }

    if let Some(error) = text.error {
        ui.glyph(content.x, y, Icon::Warning, t.danger, t.raised);
        let style = Style::new().fg(t.danger).bg(t.raised);
        ui.text(content.x + ICON_WIDTH + 1, y, content.width.saturating_sub(ICON_WIDTH + 1), error, style);
        y += error_rows;
    }

    let button_y = y.min(inner.bottom().saturating_sub(2));
    let confirm_width = pill_width(None, text.confirm);
    let cancel = &dict.importer.cancel;
    let confirm_x = content.right().saturating_sub(confirm_width);
    if !text.busy {
        let cancel_x = confirm_x.saturating_sub(pill_width(None, cancel) + 2);
        ui.pill(cancel_x, button_y, None, cancel, PillStyle::Secondary, t.raised, Action::CloseOverlay);
    }
    if text.danger {
        let button = Rect::new(confirm_x, button_y, confirm_width, 1);
        let fill = if app.hovered(button) { t.text } else { t.danger };
        ui.rounded(button, fill, t.raised);
        let style = Style::new().fg(t.bg).bg(fill).add_modifier(Modifier::BOLD);
        ui.text(confirm_x + 2, button_y, confirm_width - 4, text.confirm, style);
        ui.hits.click(button, Action::DialogSubmit);
    } else {
        ui.pill(confirm_x, button_y, None, text.confirm, PillStyle::Primary, t.raised, Action::DialogSubmit);
    }
}

pub fn toasts(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let mut bottom = area.bottom().saturating_sub(1);
    for (index, toast) in app.toasts.iter().enumerate().rev() {
        let width = (display_width(&toast.text) as u16 + ICON_WIDTH + 7).min(TOAST_MAX_WIDTH).min(area.width);
        let box_area = Rect::new(area.right().saturating_sub(width + 2), bottom.saturating_sub(3), width, 3);
        if box_area.y < area.y {
            break;
        }
        let inner = panel(ui, box_area);
        let (icon, color) = if toast.error { (Icon::Warning, t.danger) } else { (Icon::Check, t.accent) };
        ui.glyph(inner.x + 1, inner.y, icon, color, t.raised);
        let text_x = inner.x + ICON_WIDTH + 2;
        ui.text(text_x, inner.y, inner.right().saturating_sub(text_x + 1), &toast.text, Style::new().fg(t.text).bg(t.raised));
        ui.hits.click(box_area, Action::DismissToast(index));
        bottom = box_area.y;
    }
}
