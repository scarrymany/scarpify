//! The frame around pages: title bar, sidebar, queue panel and player bar.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};

use super::{clock, display_width, tail, Fallback, Ui, BUTTON_WIDTH, ICON_WIDTH};
use crate::library::SavedCollection;
use crate::model::Track;
use crate::tui::app::{Action, Drag, Field, Hit, Route, ScrollId, Slider, Source, Status, AUTHOR_URL};
use crate::tui::paint::{self, Icon, Visual};
use crate::tui::queue::Repeat;
use crate::tui::theme::provider_color;

const SEARCH_MAX_WIDTH: u16 = 64;
const ROUND_WIDTH: u16 = 4;
const VOLUME_WIDTH: u16 = 14;
/// A library entry: a 4×2 cover (40 px square) with two lines of text, then a blank row.
const ENTRY_HEIGHT: u16 = 2;
const ENTRY_STEP: u16 = 3;
const ENTRY_COVER: u16 = 4;
/// A queue or player item: a 6×3 cover (60 px square) beside title, artists and badge.
const ITEM_HEIGHT: u16 = 3;
const ITEM_STEP: u16 = 4;
const ITEM_COVER: u16 = 6;

pub fn titlebar(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let mut x = area.x + 2;
    for (icon, enabled, action) in [(Icon::Back, app.can_back(), Action::Back), (Icon::Forward, app.can_forward(), Action::Forward)] {
        let button = Rect::new(x, area.y, ROUND_WIDTH, area.height);
        if enabled {
            ui.round_button(button, icon, t.bg, t.muted, t.bg, action);
        } else {
            let [fill, faint] = [paint::rgb(t.bg), paint::rgb(t.faint)];
            if !ui.picture(button, Visual::Round { icon, fill, fg: faint, bg: fill }) {
                ui.text(button.x + 1, button.y + 1, ICON_WIDTH, icon.glyph(), Style::new().fg(t.faint).bg(t.bg));
            }
        }
        x += ROUND_WIDTH + 1;
    }

    let width = SEARCH_MAX_WIDTH.min(area.width / 2);
    let search = Rect::new(area.x + (area.width - width) / 2, area.y, width, area.height);
    let focused = app.focus == Some(Field::Search);
    let fill = if focused {
        t.active
    } else if app.hovered(search) {
        t.hover
    } else {
        t.raised
    };
    ui.rounded(search, fill, t.bg);
    ui.hits.add(Hit { area: search, click: Some(Action::FocusSearch), field: Some(Field::Search), ..Hit::default() });

    let y = search.y + search.height / 2;
    let on_search = *app.route() == Route::Search || focused;
    ui.glyph(search.x + 2, y, if on_search { Icon::SearchBold } else { Icon::Search }, if on_search { t.text } else { t.muted }, fill);
    let text_x = search.x + 2 + ICON_WIDTH + 1;
    let text_width = search.right().saturating_sub(text_x + BUTTON_WIDTH + 2);
    let query = &app.search.query;
    if query.is_empty() && !focused {
        ui.text(text_x, y, text_width, &app.dict.search.placeholder, Style::new().fg(t.faint).bg(fill));
    } else {
        let shown = tail(query, text_width.saturating_sub(1) as usize);
        let used = ui.text(text_x, y, text_width, &shown, Style::new().fg(t.text).bg(fill));
        if focused {
            ui.text(text_x + used, y, 1, "▏", Style::new().fg(t.accent).bg(fill));
        }
    }
    if !query.is_empty() {
        ui.icon_button(search.right() - BUTTON_WIDTH - 1, y, Icon::Close, t.muted, fill, Action::ClearSearch);
    }

    let telegram = Rect::new(area.right().saturating_sub(ROUND_WIDTH + 2), area.y, ROUND_WIDTH, area.height);
    ui.round_button(telegram, Icon::Telegram, t.bg, t.muted, t.bg, Action::OpenUrl(AUTHOR_URL.into()));
}

fn nav_item(ui: &mut Ui, area: Rect, icons: (Icon, Icon), label: &str, route: Route) {
    let t = ui.theme();
    let current = *ui.app.route() == route;
    let hovered = ui.app.hovered(area);
    let bg = if hovered { t.hover } else { t.surface };
    if hovered {
        ui.rounded(area, t.hover, t.surface);
    }
    let (icon, color) = if current { (icons.1, t.text) } else { (icons.0, if hovered { t.text } else { t.muted }) };
    ui.glyph(area.x + 2, area.y, icon, color, bg);
    let weight = if current { Modifier::BOLD } else { Modifier::empty() };
    let text_x = area.x + 2 + ICON_WIDTH + 2;
    ui.text(text_x, area.y, area.right().saturating_sub(text_x + 1), label, Style::new().fg(color).bg(bg).add_modifier(weight));
    ui.hits.click(area, Action::Go(route));
}

pub fn sidebar(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let [primary, _, library] =
        Layout::vertical([Constraint::Length(6), Constraint::Length(1), Constraint::Min(6)]).areas(area);

    ui.rounded(primary, t.surface, t.bg);
    let wordmark = Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::BOLD);
    ui.text(primary.x + 3, primary.y + 1, primary.width - 6, "SCARPIFY", wordmark);
    let row = |y| Rect::new(primary.x + 1, y, primary.width - 2, 1);
    nav_item(ui, row(primary.y + 3), (Icon::Home, Icon::HomeFill), &app.dict.nav.home, Route::Home);
    nav_item(ui, row(primary.y + 4), (Icon::Search, Icon::SearchBold), &app.dict.nav.search, Route::Search);

    ui.rounded(library, t.surface, t.bg);
    let header_y = library.y + 1;
    ui.glyph(library.x + 3, header_y, Icon::Library, t.muted, t.surface);
    let header_style = Style::new().fg(t.muted).bg(t.surface).add_modifier(Modifier::BOLD);
    let header_x = library.x + 3 + ICON_WIDTH + 2;
    ui.text(header_x, header_y, library.right().saturating_sub(header_x + BUTTON_WIDTH + 2), &app.dict.nav.library, header_style);
    ui.icon_button(library.right() - BUTTON_WIDTH - 2, header_y, Icon::Plus, t.muted, t.surface, Action::AddMenu);

    let list = Rect::new(library.x, library.y + 3, library.width, library.height.saturating_sub(6));
    library_entries(ui, list);
    let settings_row = Rect::new(library.x + 1, library.bottom() - 2, library.width - 2, 1);
    nav_item(ui, settings_row, (Icon::Gear, Icon::GearFill), &app.dict.nav.settings, Route::Settings);
}

/// A row in a list panel: cover, title and a second line, with hover and current states.
fn entry(ui: &mut Ui, area: Rect, cover: (Option<&str>, Fallback), title: &str, current: bool) -> Color {
    let t = ui.theme();
    let bg = if current {
        t.active
    } else if ui.app.hovered(area) {
        t.hover
    } else {
        t.surface
    };
    if bg != t.surface {
        ui.rounded(area, bg, t.surface);
    }
    ui.cover(Rect::new(area.x + 1, area.y, ENTRY_COVER, ENTRY_HEIGHT), cover.0, cover.1, bg);
    let text_x = area.x + ENTRY_COVER + 3;
    ui.text(text_x, area.y, area.right().saturating_sub(text_x + 1), title, Style::new().fg(t.text).bg(bg));
    bg
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
        let area = Rect::new(list.x + 1, list.y + slot as u16 * ENTRY_STEP, list.width - 2, ENTRY_HEIGHT);
        let text_x = area.x + ENTRY_COVER + 3;
        let text_width = area.right().saturating_sub(text_x + 1);
        if index == 0 {
            let bg = entry(ui, area, (None, Fallback::Liked), &app.dict.nav.liked, *app.route() == Route::Liked);
            ui.text(text_x, area.y + 1, text_width, &app.dict.tracks(app.library.liked.len()), Style::new().fg(t.faint).bg(bg));
            ui.hits.click(area, Action::Go(Route::Liked));
            continue;
        }

        let position = index - 1;
        let collection = &app.library.collections[position];
        let route = Route::Collection(collection.id.clone());
        let bg = entry(ui, area, (cover_of(collection), Fallback::Notes), &collection.name, *app.route() == route);
        let mut x = text_x;
        let y = area.y + 1;
        if collection.pinned {
            ui.glyph(x, y, Icon::Pin, t.accent, bg);
            x += ICON_WIDTH + 1;
        }
        if let Some(provider) = collection.provider {
            ui.glyph(x, y, Icon::provider(provider), provider_color(provider), bg);
            x += ICON_WIDTH + 1;
        }
        ui.text(x, y, area.right().saturating_sub(x + 1), &app.dict.tracks(collection.tracks.len()), Style::new().fg(t.faint).bg(bg));
        ui.hits.add(Hit {
            area,
            click: Some(Action::Go(route)),
            context: Some(Action::CollectionMenu(collection.id.clone())),
            drag: Some(Drag::Collection(position)),
            ..Hit::default()
        });
    }
}

/// The cover the window shows for a playlist: its own, the service's, or the first track's.
pub fn cover_of(collection: &SavedCollection) -> Option<&str> {
    collection
        .custom_artwork
        .as_deref()
        .or(collection.artwork.as_deref())
        .or_else(|| collection.tracks.first().and_then(|t| t.artwork.as_deref()))
}

/// Cover, title, artists and the service badge, as in the queue and the player bar.
fn track_item(ui: &mut Ui, area: Rect, track: &Track, highlight: bool, bg: Color) {
    let t = ui.theme();
    ui.cover(Rect::new(area.x, area.y, ITEM_COVER, ITEM_HEIGHT), track.artwork.as_deref(), Fallback::Notes, bg);
    let text_x = area.x + ITEM_COVER + 2;
    let width = area.right().saturating_sub(text_x);
    let title = if highlight {
        Style::new().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(t.text).bg(bg).add_modifier(Modifier::BOLD)
    };
    ui.text(text_x, area.y, width, &track.title, title);
    ui.text(text_x, area.y + 1, width, &track.artists.join(", "), Style::new().fg(t.muted).bg(bg));
    ui.badge(text_x, area.y + 2, width, track.provider, bg);
}

pub fn queue(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    ui.rounded(area, t.surface, t.bg);
    let inner_x = area.x + 3;
    let inner_width = area.width.saturating_sub(6);
    ui.heading(inner_x, area.y + 1, inner_width.saturating_sub(BUTTON_WIDTH), &app.dict.queue.title, t.surface);
    ui.icon_button(area.right() - BUTTON_WIDTH - 2, area.y + 1, Icon::Close, t.muted, t.surface, Action::ToggleQueue);

    let small = Style::new().fg(t.muted).bg(t.surface).add_modifier(Modifier::BOLD);
    let Some(current) = app.queue.current() else {
        ui.text(inner_x, area.y + 3, inner_width, &app.dict.queue.empty, Style::new().fg(t.faint).bg(t.surface));
        return;
    };
    let current_index = app.queue.index().unwrap_or(0);
    ui.text(inner_x, area.y + 3, inner_width, &app.dict.queue.now_playing, small);
    let current_row = Rect::new(area.x + 1, area.y + 5, area.width - 2, ITEM_HEIGHT);
    let bg = if app.hovered(current_row) { t.hover } else { t.surface };
    if bg != t.surface {
        ui.rounded(current_row, bg, t.surface);
    }
    track_item(ui, Rect::new(current_row.x + 1, current_row.y, current_row.width - 2, ITEM_HEIGHT), current, true, bg);
    ui.hits.click(current_row, Action::TogglePlay);

    let next_y = current_row.bottom() + 1;
    ui.text(inner_x, next_y, inner_width, &app.dict.queue.next, small);
    let upcoming = app.queue.upcoming();
    if !upcoming.is_empty() {
        let clear = &app.dict.queue.clear;
        let clear_width = display_width(clear) as u16 + 2;
        let clear_area = Rect::new(area.right() - clear_width - 2, next_y, clear_width, 1);
        let hovered = app.hovered(clear_area);
        let style = if hovered {
            Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::new().fg(t.muted).bg(t.surface)
        };
        ui.text(clear_area.x + 1, next_y, clear_width - 2, clear, style);
        ui.hits.click(clear_area, Action::ClearUpcoming);
        if upcoming.len() > 1 {
            ui.icon_button(clear_area.x - BUTTON_WIDTH, next_y, Icon::Shuffle, t.muted, t.surface, Action::ShuffleUpcoming);
        }
    }

    let list = Rect::new(area.x, next_y + 2, area.width, area.bottom().saturating_sub(next_y + 3));
    if upcoming.is_empty() {
        ui.text(inner_x, list.y, inner_width, &app.dict.queue.empty, Style::new().fg(t.faint).bg(t.surface));
        return;
    }
    let visible = (list.height / ITEM_STEP) as usize;
    let max_offset = upcoming.len().saturating_sub(visible);
    ui.hits.scroll(list, ScrollId::Queue, max_offset);
    let offset = app.offset(ScrollId::Queue).min(max_offset);
    for (slot, position) in (offset..upcoming.len()).take(visible).enumerate() {
        let row = Rect::new(area.x + 1, list.y + slot as u16 * ITEM_STEP, area.width - 2, ITEM_HEIGHT);
        let bg = if app.hovered(row) { t.hover } else { t.surface };
        if bg != t.surface {
            ui.rounded(row, bg, t.surface);
        }
        track_item(ui, Rect::new(row.x + 1, row.y, row.width - 2, ITEM_HEIGHT), &upcoming[position], false, bg);
        let absolute = current_index + 1 + position;
        ui.hits.add(Hit {
            area: row,
            double: Some(Action::Play(Source::Queue, absolute)),
            context: Some(Action::TrackMenu(Source::Queue, absolute)),
            drag: Some(Drag::Upcoming(position)),
            ..Hit::default()
        });
    }
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
        ui.text(area.x + 3, area.y + 2, area.width.saturating_sub(4), &app.dict.player.nothing, Style::new().fg(t.faint).bg(t.bg));
        return;
    };
    let item = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(BUTTON_WIDTH + 4), ITEM_HEIGHT);
    track_item(ui, item, track, false, t.bg);
    let liked = app.is_liked(track);
    let (icon, color) = if liked { (Icon::HeartFill, t.accent) } else { (Icon::Heart, t.muted) };
    ui.icon_button(area.right().saturating_sub(BUTTON_WIDTH + 1), area.y + 2, icon, color, t.bg, Action::ToggleLike(track.clone()));
}

fn controls(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let has_track = app.queue.current().is_some();
    let toggled = |on: bool| if on { t.accent } else { t.muted };
    let enabled = |yes: bool| if yes { t.muted } else { t.faint };
    let repeat = if app.queue.repeat == Repeat::One { Icon::RepeatOnce } else { Icon::Repeat };
    // Loading keeps the triangle, so skipping never flashes the pause bars.
    let play = if app.status == Status::Playing { Icon::Pause } else { Icon::Play };

    let buttons = [
        (Icon::Shuffle, toggled(app.queue.shuffle), Action::ToggleShuffle),
        (Icon::Previous, enabled(has_track), Action::Previous),
        (play, t.bg, Action::TogglePlay),
        (Icon::Next, enabled(has_track), Action::Next),
        (repeat, toggled(app.queue.repeat != Repeat::Off), Action::CycleRepeat),
    ];
    let row_width = buttons.len() as u16 * ROUND_WIDTH + (buttons.len() as u16 - 1) * 2;
    let mut x = area.x + area.width.saturating_sub(row_width) / 2;
    for (icon, color, action) in buttons {
        let button = Rect::new(x, area.y + 1, ROUND_WIDTH, 2);
        if icon == play {
            let fill = if has_track { t.text } else { t.faint };
            ui.round_button(button, icon, fill, t.bg, t.bg, action);
        } else {
            ui.round_button(button, icon, t.bg, color, t.bg, action);
        }
        x += ROUND_WIDTH + 2;
    }

    let duration = app.queue.current().and_then(|track| track.duration_ms);
    let shown = app.scrub_ms.unwrap_or(app.position_ms);
    let elapsed = clock(has_track.then_some(shown));
    let total = clock(duration);
    let bar_y = area.y + 3;
    let time_style = Style::new().fg(t.faint).bg(t.bg);
    let left_width = display_width(&elapsed) as u16;
    ui.text(area.x + 1, bar_y, left_width, &elapsed, time_style);
    let right_width = display_width(&total) as u16;
    ui.text(area.right().saturating_sub(right_width + 1), bar_y, right_width, &total, time_style);
    let bar = Rect::new(area.x + left_width + 2, bar_y, area.width.saturating_sub(left_width + right_width + 4), 1);
    let ratio = match duration {
        Some(duration) if duration > 0 => shown as f64 / duration as f64,
        _ => 0.0,
    };
    let seekable = has_track && duration.is_some_and(|d| d > 0) && app.status != Status::Loading;
    ui.slider(bar, ratio, Slider::Seek, seekable, t.bg);
}

fn extras(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let y = area.y + 2;
    let slider = Rect::new(area.right().saturating_sub(VOLUME_WIDTH + 2), y, VOLUME_WIDTH, 1);
    let volume_icon = if app.muted() {
        Icon::Muted
    } else if app.volume() < 0.5 {
        Icon::VolumeLow
    } else {
        Icon::VolumeHigh
    };
    let volume_x = slider.x - BUTTON_WIDTH - 1;
    ui.icon_button(volume_x, y, volume_icon, t.muted, t.bg, Action::ToggleMute);
    let queue_color = if app.queue_open { t.accent } else { t.muted };
    ui.icon_button(volume_x - BUTTON_WIDTH - 1, y, Icon::Queue, queue_color, t.bg, Action::ToggleQueue);
    let shown = if app.muted() { 0.0 } else { app.volume() as f64 };
    ui.slider(slider, shown, Slider::Volume, true, t.bg);
}
