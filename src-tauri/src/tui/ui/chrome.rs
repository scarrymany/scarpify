//! The frame around pages: title bar, sidebar, queue panel and player bar.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::{clock, display_width, ellipsize, tail, Fallback, Ui};
use crate::library::SavedCollection;
use crate::model::Track;
use crate::tui::app::{Action, Drag, Field, Hit, Route, ScrollId, Slider, Source, Status, AUTHOR_URL};
use crate::tui::queue::Repeat;
use crate::tui::theme::provider_color;

const ENTRY_STEP: u16 = 3;
const ART_WIDTH: u16 = 4;
const ART_HEIGHT: u16 = 2;
const SEARCH_MAX_WIDTH: u16 = 64;
const VOLUME_WIDTH: u16 = 14;

/// A cover with a title and a second line, as in the library list and the queue.
struct Entry<'a> {
    art: Option<&'a str>,
    fallback: Fallback,
    title: &'a str,
    highlight: bool,
    subtitle: Line<'a>,
}

fn entry(ui: &mut Ui, area: Rect, item: Entry, current: bool) {
    let t = ui.theme();
    let bg = if current {
        t.active
    } else if ui.app.hovered(area) {
        t.hover
    } else {
        t.surface
    };
    ui.fill(area, bg);
    ui.art(Rect::new(area.x + 1, area.y, ART_WIDTH, ART_HEIGHT), item.art, item.fallback);
    let text_x = area.x + ART_WIDTH + 2;
    let width = area.right().saturating_sub(text_x + 1);
    let title_style = if item.highlight {
        Style::new().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(t.text).bg(bg)
    };
    ui.text(text_x, area.y, width, item.title, title_style);
    ui.line(Rect::new(text_x, area.y + 1, width, 1), item.subtitle.style(Style::new().bg(bg)));
}

pub fn titlebar(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let y = area.y + 1;
    let mut x = area.x + 2;
    for (glyph, enabled, action) in [("‹", app.can_back(), Action::Back), ("›", app.can_forward(), Action::Forward)] {
        if enabled {
            x += ui.icon(x, y, glyph, false, action) + 1;
        } else {
            ui.text(x, y, 3, &format!(" {glyph} "), Style::new().fg(t.faint));
            x += 4;
        }
    }

    let width = SEARCH_MAX_WIDTH.min(area.width / 2);
    let search = Rect::new(area.x + (area.width - width) / 2, area.y, width, area.height);
    let focused = app.focus == Some(Field::Search);
    let on_search = *app.route() == Route::Search;
    let bg = if focused {
        t.active
    } else if app.hovered(search) {
        t.hover
    } else {
        t.raised
    };
    ui.fill(search, bg);
    ui.hits.add(Hit { area: search, click: Some(Action::FocusSearch), field: Some(Field::Search), ..Hit::default() });

    let icon_style = Style::new().fg(if on_search || focused { t.text } else { t.muted }).bg(bg);
    let used = ui.text(search.x + 2, y, 3, t.icons.search, icon_style);
    let text_x = search.x + 3 + used;
    let query = &app.search.query;
    let text_width = search.right().saturating_sub(text_x + 4);
    if query.is_empty() && !focused {
        ui.text(text_x, y, text_width, &app.dict.search.placeholder, Style::new().fg(t.faint).bg(bg));
    } else {
        // Long queries scroll left so the end being typed stays visible.
        let shown = tail(query, text_width.saturating_sub(1) as usize);
        let used = ui.text(text_x, y, text_width, &shown, Style::new().fg(t.text).bg(bg));
        if focused {
            ui.text(text_x + used, y, 1, "▏", Style::new().fg(t.accent).bg(bg));
        }
    }
    if !query.is_empty() {
        ui.icon(search.right() - 4, y, "✕", false, Action::ClearSearch);
    }

    let telegram = t.icons.telegram;
    let telegram_width = display_width(telegram) as u16 + 2;
    ui.icon(area.right().saturating_sub(telegram_width + 2), y, telegram, false, Action::OpenUrl(AUTHOR_URL.into()));
}

fn nav_item(ui: &mut Ui, area: Rect, icon: &str, label: &str, route: Route) {
    let t = ui.theme();
    let current = *ui.app.route() == route;
    let style = match (current, ui.app.hovered(area)) {
        (true, _) => Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::BOLD),
        (false, true) => Style::new().fg(t.text).bg(t.hover),
        (false, false) => Style::new().fg(t.muted).bg(t.surface),
    };
    ui.button(area, Line::from(format!("  {icon}  {label}")), style, style, Action::Go(route));
}

pub fn sidebar(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let [primary, _, library] =
        Layout::vertical([Constraint::Length(6), Constraint::Length(1), Constraint::Min(5)]).areas(area);

    ui.fill(primary, t.surface);
    let wordmark = Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::BOLD);
    ui.text(primary.x + 2, primary.y + 1, primary.width - 4, "SCARPIFY", wordmark);
    let row = |y| Rect::new(primary.x + 1, y, primary.width - 2, 1);
    nav_item(ui, row(primary.y + 3), t.icons.home, &app.dict.nav.home, Route::Home);
    nav_item(ui, row(primary.y + 4), t.icons.search, &app.dict.nav.search, Route::Search);

    ui.fill(library, t.surface);
    let header = format!("{}  {}", t.icons.library, app.dict.nav.library);
    let header_style = Style::new().fg(t.muted).bg(t.surface).add_modifier(Modifier::BOLD);
    ui.text(library.x + 2, library.y + 1, library.width - 8, &header, header_style);
    let add_width = display_width(t.icons.add) as u16 + 2;
    ui.icon(library.right() - add_width - 1, library.y + 1, t.icons.add, false, Action::AddMenu);

    let list = Rect::new(library.x, library.y + 3, library.width, library.height.saturating_sub(6));
    library_entries(ui, list);
    let settings_row = Rect::new(library.x + 1, library.bottom() - 2, library.width - 2, 1);
    nav_item(ui, settings_row, t.icons.settings, &app.dict.nav.settings, Route::Settings);
}

fn library_entries(ui: &mut Ui, list: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let total = app.library.collections.len() + 1;
    let visible = (list.height / ENTRY_STEP) as usize;
    let max_offset = total.saturating_sub(visible);
    ui.hits.scroll(list, ScrollId::Sidebar, max_offset);
    let offset = app.offset(ScrollId::Sidebar).min(max_offset);

    for (slot, index) in (offset..total).take(visible).enumerate() {
        let area = Rect::new(list.x + 1, list.y + slot as u16 * ENTRY_STEP, list.width - 2, ART_HEIGHT);
        if index == 0 {
            let subtitle = Line::styled(app.dict.tracks(app.library.liked.len()), Style::new().fg(t.faint));
            let item = Entry { art: None, fallback: Fallback::Liked, title: &app.dict.nav.liked, highlight: false, subtitle };
            entry(ui, area, item, *app.route() == Route::Liked);
            ui.hits.click(area, Action::Go(Route::Liked));
            continue;
        }

        let position = index - 1;
        let collection = &app.library.collections[position];
        let mut spans = Vec::new();
        if collection.pinned {
            spans.push(Span::raw(format!("{} ", t.icons.pin)));
        }
        if let Some(provider) = collection.provider {
            spans.push(Span::styled("● ", Style::new().fg(provider_color(provider))));
        }
        spans.push(Span::styled(app.dict.tracks(collection.tracks.len()), Style::new().fg(t.faint)));
        let route = Route::Collection(collection.id.clone());
        let current = *app.route() == route;
        let item = Entry {
            art: cover_of(collection),
            fallback: Fallback::Note,
            title: &collection.name,
            highlight: false,
            subtitle: Line::from(spans),
        };
        entry(ui, area, item, current);
        ui.hits.add(Hit {
            area,
            click: Some(Action::Go(route)),
            context: Some(Action::CollectionMenu(collection.id.clone())),
            drag: Some(Drag::Collection(position)),
            ..Hit::default()
        });
    }
}

pub fn cover_of(collection: &SavedCollection) -> Option<&str> {
    collection
        .custom_artwork
        .as_deref()
        .or(collection.artwork.as_deref())
        .or_else(|| collection.tracks.first().and_then(|t| t.artwork.as_deref()))
        // Covers chosen in the window are stored inline as data URLs; only links are fetched.
        .filter(|url| url.starts_with("http"))
}

pub fn queue(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    ui.fill(area, t.surface);
    let inner_x = area.x + 2;
    let inner_width = area.width.saturating_sub(4);
    ui.heading(inner_x, area.y + 1, inner_width.saturating_sub(4), &app.dict.queue.title);
    ui.icon(area.right() - 5, area.y + 1, "✕", false, Action::ToggleQueue);

    let Some(current) = app.queue.current() else {
        ui.text(inner_x, area.y + 3, inner_width, &app.dict.queue.empty, Style::new().fg(t.faint));
        return;
    };
    let current_index = app.queue.index().unwrap_or(0);
    let small = Style::new().fg(t.muted).add_modifier(Modifier::BOLD);
    ui.text(inner_x, area.y + 3, inner_width, &app.dict.queue.now_playing, small);
    let current_area = Rect::new(area.x + 1, area.y + 4, area.width - 2, ART_HEIGHT);
    queue_item(ui, current_area, current, true);
    ui.hits.click(current_area, Action::TogglePlay);

    let next_y = area.y + 7;
    ui.text(inner_x, next_y, inner_width, &app.dict.queue.next, small);
    let upcoming = app.queue.upcoming();
    if !upcoming.is_empty() {
        let clear = &app.dict.queue.clear;
        let clear_width = display_width(clear) as u16 + 2;
        let clear_area = Rect::new(area.right() - clear_width - 2, next_y, clear_width, 1);
        let normal = Style::new().fg(t.muted);
        let hover = Style::new().fg(t.text).add_modifier(Modifier::UNDERLINED);
        ui.button(clear_area, Line::from(format!(" {clear} ")), normal, hover, Action::ClearUpcoming);
        if upcoming.len() > 1 {
            ui.icon(clear_area.x - 5, next_y, "⇄", false, Action::ShuffleUpcoming);
        }
    }

    let list = Rect::new(area.x, next_y + 2, area.width, area.bottom().saturating_sub(next_y + 2));
    if upcoming.is_empty() {
        ui.text(inner_x, list.y, inner_width, &app.dict.queue.empty, Style::new().fg(t.faint));
        return;
    }
    let visible = (list.height / ENTRY_STEP) as usize;
    let max_offset = upcoming.len().saturating_sub(visible);
    ui.hits.scroll(list, ScrollId::Queue, max_offset);
    let offset = app.offset(ScrollId::Queue).min(max_offset);
    for (slot, position) in (offset..upcoming.len()).take(visible).enumerate() {
        let item_area = Rect::new(area.x + 1, list.y + slot as u16 * ENTRY_STEP, area.width - 2, ART_HEIGHT);
        queue_item(ui, item_area, &upcoming[position], false);
        let absolute = current_index + 1 + position;
        ui.hits.add(Hit {
            area: item_area,
            double: Some(Action::Play(Source::Queue, absolute)),
            context: Some(Action::TrackMenu(Source::Queue, absolute)),
            drag: Some(Drag::Upcoming(position)),
            ..Hit::default()
        });
    }
}

fn queue_item(ui: &mut Ui, area: Rect, track: &Track, active: bool) {
    let t = ui.theme();
    let subtitle = Line::from(vec![
        Span::styled("● ", Style::new().fg(provider_color(track.provider))),
        Span::styled(track.artists.join(", "), Style::new().fg(t.muted)),
    ]);
    let item = Entry { art: track.artwork.as_deref(), fallback: Fallback::Note, title: &track.title, highlight: active, subtitle };
    entry(ui, area, item, false);
}

pub fn playerbar(ui: &mut Ui, area: Rect) {
    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .areas(area);
    now_playing(ui, left);
    controls(ui, center);
    extras(ui, right);
}

fn now_playing(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let Some(track) = app.queue.current() else {
        ui.text(area.x + 3, area.y + 2, area.width.saturating_sub(4), &app.dict.player.nothing, Style::new().fg(t.faint));
        return;
    };
    let art = Rect::new(area.x + 2, area.y + 1, 6, 3);
    ui.art(art, track.artwork.as_deref(), Fallback::Note);
    let text_x = art.right() + 2;
    let width = area.right().saturating_sub(text_x + 5);
    ui.text(text_x, area.y + 1, width, &track.title, Style::new().fg(t.text).add_modifier(Modifier::BOLD));
    ui.text(text_x, area.y + 2, width, &track.artists.join(", "), Style::new().fg(t.muted));
    ui.badge(text_x, area.y + 3, width, track.provider);

    let liked = app.is_liked(track);
    let heart = if liked { "♥" } else { "♡" };
    ui.icon(area.right().saturating_sub(4), area.y + 2, heart, liked, Action::ToggleLike(track.clone()));
}

fn controls(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let has_track = app.queue.current().is_some();
    let repeat = if app.queue.repeat == Repeat::One { "↻¹" } else { "↻" };
    // Loading keeps the triangle, so skipping never flashes the pause bars.
    let play = if app.status == Status::Playing { "⏸" } else { "▶" };
    let row_width = 4 * 4 + 6;
    let mut x = area.x + area.width.saturating_sub(row_width) / 2;
    let y = area.y + 1;

    x += ui.icon(x, y, "⇄", app.queue.shuffle, Action::ToggleShuffle) + 1;
    x += ui.icon(x, y, "⏮", false, Action::Previous) + 1;
    let play_area = Rect::new(x, y, 5, 1);
    let bg = match (has_track, ui.app.hovered(play_area)) {
        (false, _) => t.faint,
        (true, true) => t.accent,
        (true, false) => t.text,
    };
    let style = Style::new().fg(t.bg).bg(bg).add_modifier(Modifier::BOLD);
    ui.button(play_area, Line::from(format!("  {play}  ")), style, style, Action::TogglePlay);
    x += 6;
    x += ui.icon(x, y, "⏭", false, Action::Next) + 1;
    ui.icon(x, y, repeat, app.queue.repeat != Repeat::Off, Action::CycleRepeat);

    let duration = app.queue.current().and_then(|track| track.duration_ms);
    let shown = app.scrub_ms.unwrap_or(app.position_ms);
    let elapsed = clock(has_track.then_some(shown));
    let total = clock(duration);
    let bar_y = area.y + 3;
    let time_style = Style::new().fg(t.faint);
    let left_width = display_width(&elapsed) as u16;
    ui.text(area.x + 1, bar_y, left_width, &elapsed, time_style);
    let right_width = display_width(&total) as u16;
    ui.text(area.right().saturating_sub(right_width + 1), bar_y, right_width, &total, time_style);
    let bar = Rect::new(area.x + left_width + 2, bar_y, area.width.saturating_sub(left_width + right_width + 4), 1);
    let ratio = match duration {
        Some(duration) if duration > 0 => shown as f64 / duration as f64,
        _ => 0.0,
    };
    let enabled = has_track && duration.is_some_and(|d| d > 0) && app.status != Status::Loading;
    ui.slider(bar, ratio, Slider::Seek, enabled);
    if app.status == Status::Loading {
        let loading = ellipsize(&app.dict.player.loading, bar.width as usize);
        let x = bar.x + bar.width.saturating_sub(display_width(&loading) as u16) / 2;
        ui.text(x, area.y + 2, bar.width, &loading, time_style);
    }
}

fn extras(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let y = area.y + 2;
    let slider = Rect::new(area.right().saturating_sub(VOLUME_WIDTH + 2), y, VOLUME_WIDTH, 1);
    let volume_icon = if app.muted() {
        t.icons.muted
    } else if app.volume() < 0.5 {
        t.icons.volume_low
    } else {
        t.icons.volume_high
    };
    let icon_width = display_width(volume_icon) as u16 + 2;
    ui.icon(slider.x - icon_width - 1, y, volume_icon, false, Action::ToggleMute);
    let queue_width = display_width(t.icons.queue) as u16 + 2;
    ui.icon(slider.x - icon_width - queue_width - 2, y, t.icons.queue, app.queue_open, Action::ToggleQueue);
    let shown = if app.muted() { 0.0 } else { app.volume() as f64 };
    ui.slider(slider, shown, Slider::Volume, true);
}
