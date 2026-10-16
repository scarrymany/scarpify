//! The page in the middle: home, search, a playlist or album, liked tracks, settings.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Paragraph, Wrap};

use super::chrome::cover_of;
use super::{clock, display_width, ellipsize, pill_width, segment_width, Fallback, PillStyle, Ui, BUTTON_WIDTH, ICON_WIDTH};
use crate::library::{Origin, SavedCollection};
use crate::model::{CollectionKind, Provider, Track};
use crate::mode::{Theme as ThemeChoice, ACCENTS};
use crate::tui::app::{Action, Drag, Filter, Hit, Route, ScrollId, Source, Status, REPOSITORY_URL};
use crate::tui::i18n::{format, LOCALES};
use crate::tui::paint::{self, Icon, Visual};
use crate::tui::queue::key_of;
use crate::tui::theme::{local_hour, parse_hex};

const PAD_X: u16 = 3;
const CARD_WIDTH: u16 = 22;
const CARD_GAP: u16 = 2;
const CARD_COVER_HEIGHT: u16 = 10;
const CARD_HEIGHT: u16 = CARD_COVER_HEIGHT + 6;
/// Track rows: a 6×3 cover (60 px) beside title, artists and the service badge.
const ROW_HEIGHT: u16 = 3;
const ROW_COVER: u16 = 6;
const HERO_COVER_WIDTH: u16 = 20;
const HERO_COVER_HEIGHT: u16 = 10;
const SKELETON_ROWS: usize = 6;
const ALBUM_MIN_WIDTH: u16 = 100;
const SETTINGS_HEIGHT: i32 = 44;
const SETTINGS_WIDTH: u16 = 100;
const CONTROL_COLUMN: u16 = 34;
const SWATCH_STEP: u16 = 3;

pub fn main(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    ui.rounded(area, t.surface, t.bg);
    let inner = Rect::new(area.x + PAD_X, area.y + 1, area.width.saturating_sub(PAD_X * 2), area.height.saturating_sub(2));
    match app.route() {
        Route::Home => home(ui, inner),
        Route::Search => search(ui, inner),
        Route::Liked => collection(ui, inner, None),
        Route::Collection(id) => match app.collection(id) {
            Some(saved) => collection(ui, inner, Some(saved)),
            None => home(ui, inner),
        },
        Route::Settings => settings(ui, inner),
    }
}

fn greeting(ui: &Ui) -> String {
    let home = &ui.app.dict.home;
    match local_hour() {
        5..=11 => home.morning.clone(),
        12..=17 => home.afternoon.clone(),
        18..=22 => home.evening.clone(),
        _ => home.night.clone(),
    }
}

fn title(ui: &mut Ui, x: u16, y: u16, width: u16, text: &str) {
    let t = ui.theme();
    ui.heading(x, y, width, text, t.surface);
}

fn centered_title(ui: &mut Ui, area: Rect, y: u16, text: &str) {
    let x = area.x + area.width.saturating_sub(display_width(text) as u16) / 2;
    title(ui, x, y, area.width, text);
}

fn paragraph(ui: &mut Ui, area: Rect, text: &str, centered: bool) {
    let t = ui.theme();
    let style = Style::new().fg(t.muted).bg(t.surface);
    let mut widget = Paragraph::new(text.to_owned()).style(style).wrap(Wrap { trim: true });
    if centered {
        widget = widget.alignment(Alignment::Center);
    }
    ui.frame.render_widget(widget, area);
}

fn home(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let greeting = greeting(ui);
    title(ui, area.x, area.y + 1, area.width, &greeting);

    let library = &app.library;
    if library.recent.is_empty() && library.liked.is_empty() && library.collections.is_empty() {
        let y = area.y + area.height / 3;
        centered_title(ui, area, y, &app.dict.home.empty_title);
        paragraph(ui, Rect::new(area.x, y + 2, area.width, 2), &app.dict.home.empty_body, true);
        let search = &app.dict.home.search_action;
        let import = &app.dict.importer.action;
        let total = pill_width(Some(Icon::SearchBold), search) + 2 + pill_width(None, import);
        let x = area.x + area.width.saturating_sub(total) / 2;
        let used = ui.pill(x, y + 5, Some(Icon::SearchBold), search, PillStyle::Primary, t.surface, Action::FocusSearch);
        ui.pill(x + used + 2, y + 5, None, import, PillStyle::Secondary, t.surface, Action::OpenImport);
        return;
    }

    let mut y = area.y + 4;
    if !library.recent.is_empty() {
        let cards = library
            .recent
            .iter()
            .enumerate()
            .map(|(index, track)| Card {
                cover: (track.artwork.as_deref(), Fallback::Notes),
                title: &track.title,
                subtitle: track.artists.join(", "),
                provider: Some(track.provider),
                playing: app.status == Status::Playing && app.is_current(track),
                open: None,
                play: Action::Play(Source::Recent, index),
                context: Some(Action::TrackMenu(Source::Recent, index)),
            })
            .collect();
        shelf(ui, Rect::new(area.x, y, area.width, CARD_HEIGHT + 2), 0, &app.dict.home.recent, cards);
        y += CARD_HEIGHT + 3;
    }

    let mut cards = vec![Card {
        cover: (None, Fallback::Liked),
        title: &app.dict.nav.liked,
        subtitle: app.dict.tracks(library.liked.len()),
        provider: None,
        playing: app.is_playing_from(&Source::Liked),
        open: Some(Action::Go(Route::Liked)),
        play: Action::PlayAll(Source::Liked),
        context: None,
    }];
    cards.extend(library.collections.iter().map(|collection| {
        let source = Source::Collection(collection.id.clone());
        Card {
            cover: (cover_of(collection), Fallback::Notes),
            title: &collection.name,
            subtitle: app.dict.tracks(collection.tracks.len()),
            provider: collection.provider,
            playing: app.is_playing_from(&source),
            open: Some(Action::Go(Route::Collection(collection.id.clone()))),
            play: Action::PlayAll(source),
            context: Some(Action::CollectionMenu(collection.id.clone())),
        }
    }));
    let height = area.bottom().saturating_sub(y).min(CARD_HEIGHT + 2);
    if height > CARD_COVER_HEIGHT {
        shelf(ui, Rect::new(area.x, y, area.width, height), 1, &app.dict.home.playlists, cards);
    }
}

struct Card<'a> {
    cover: (Option<&'a str>, Fallback),
    title: &'a str,
    subtitle: String,
    provider: Option<Provider>,
    playing: bool,
    /// Playlists open on click; tracks play.
    open: Option<Action>,
    play: Action,
    context: Option<Action>,
}

/// A titled row of cards that scrolls sideways with the wheel or the arrows.
fn shelf(ui: &mut Ui, area: Rect, index: usize, heading: &str, cards: Vec<Card>) {
    let t = ui.theme();
    let id = ScrollId::Shelf(index);
    let visible = ((area.width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).max(1) as usize;
    let max_offset = cards.len().saturating_sub(visible);
    let offset = ui.app.offset(id).min(max_offset);
    title(ui, area.x, area.y, area.width.saturating_sub(BUTTON_WIDTH * 2 + 2), heading);
    if max_offset > 0 {
        let x = area.right().saturating_sub(BUTTON_WIDTH * 2);
        let step = visible as isize;
        let color = |enabled: bool| if enabled { t.muted } else { t.faint };
        ui.icon_button(x, area.y, Icon::Back, color(offset > 0), t.surface, Action::Scroll(id, -step, max_offset));
        ui.icon_button(x + BUTTON_WIDTH, area.y, Icon::Forward, color(offset < max_offset), t.surface, Action::Scroll(id, step, max_offset));
    }
    let row = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(2));
    ui.hits.scroll(row, id, max_offset);
    for (slot, card) in cards.into_iter().skip(offset).take(visible).enumerate() {
        let x = row.x + slot as u16 * (CARD_WIDTH + CARD_GAP);
        card_view(ui, Rect::new(x, row.y, CARD_WIDTH, row.height.min(CARD_HEIGHT)), card);
    }
}

fn card_view(ui: &mut Ui, area: Rect, card: Card) {
    let t = ui.theme();
    let hovered = ui.app.hovered(area);
    let bg = if hovered { t.raised } else { t.surface };
    if hovered {
        ui.rounded(area, bg, t.surface);
    }
    let cover_height = CARD_COVER_HEIGHT.min(area.height.saturating_sub(5));
    ui.cover(Rect::new(area.x + 1, area.y + 1, area.width - 2, cover_height), card.cover.0, card.cover.1, bg);
    ui.hits.add(Hit {
        area,
        click: Some(card.open.clone().unwrap_or_else(|| card.play.clone())),
        context: card.context,
        ..Hit::default()
    });

    let text_y = area.y + cover_height + 2;
    let width = area.width - 2;
    // The play button sits beside the text: drawn over a cover it would cut a hole in it.
    let show_play = hovered || card.playing;
    let text_width = if show_play { width.saturating_sub(BUTTON_WIDTH + 1) } else { width };
    ui.text(area.x + 1, text_y, text_width, card.title, Style::new().fg(t.text).bg(bg).add_modifier(Modifier::BOLD));
    ui.text(area.x + 1, text_y + 1, text_width, &card.subtitle, Style::new().fg(t.muted).bg(bg));
    if let Some(provider) = card.provider {
        ui.badge(area.x + 1, text_y + 2, text_width, provider, bg);
    }
    if show_play {
        let icon = if card.playing { Icon::Pause } else { Icon::Play };
        let button = Rect::new(area.right() - BUTTON_WIDTH - 1, text_y, BUTTON_WIDTH, 2);
        ui.round_button(button, icon, t.accent, t.on_accent, bg, card.play);
    }
}

fn search(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let search = &app.search;
    if search.query.trim().is_empty() {
        let y = area.y + area.height / 3;
        centered_title(ui, area, y, &app.dict.search.idle_title);
        paragraph(ui, Rect::new(area.x, y + 2, area.width, 2), &app.dict.search.idle_body, true);
        let widths = [Provider::Youtube, Provider::Soundcloud].map(|p| display_width(app.dict.provider(p)) as u16 + ICON_WIDTH + 1);
        let x = area.x + area.width.saturating_sub(widths[0] + widths[1] + 4) / 2;
        let used = ui.badge(x, y + 5, area.width, Provider::Youtube, t.surface);
        ui.badge(x + used + 4, y + 5, area.width, Provider::Soundcloud, t.surface);
        return;
    }

    let filters = [
        (Filter::All, app.dict.search.all.as_str()),
        (Filter::Youtube, app.dict.provider(Provider::Youtube)),
        (Filter::Soundcloud, app.dict.provider(Provider::Soundcloud)),
    ];
    let mut x = area.x;
    for (filter, label) in filters {
        let style = if search.filter() == filter { PillStyle::Selected } else { PillStyle::Secondary };
        x += ui.pill(x, area.y, None, label, style, t.surface, Action::SetFilter(filter)) + 1;
    }

    let mut y = area.y + 2;
    for provider in search.shown_providers() {
        if let Some(error) = search.result(*provider).and_then(|r| r.error.as_deref()) {
            let text = format(&app.dict.search.provider_error, &[("provider", app.dict.provider(*provider)), ("error", error)]);
            ui.glyph(area.x, y, Icon::Warning, t.danger, t.surface);
            ui.text(area.x + ICON_WIDTH + 1, y, area.width.saturating_sub(ICON_WIDTH + 1), &text, Style::new().fg(t.danger).bg(t.surface));
            y += 1;
        }
    }

    let tracks = search.tracks();
    if tracks.is_empty() && search.waiting() {
        for row in 0..SKELETON_ROWS {
            let row_y = y + 1 + row as u16 * (ROW_HEIGHT + 1);
            if row_y + ROW_HEIGHT > area.bottom() {
                break;
            }
            let cover = Rect::new(area.x + 6, row_y, ROW_COVER, ROW_HEIGHT);
            ui.rounded(cover, t.raised, t.surface);
            let long = 24 + (row * 17 % 35) as u16;
            let short = 14 + (row * 11 % 20) as u16;
            ui.rounded(Rect::new(area.x + 14, row_y, long.min(area.width / 2), 1), t.raised, t.surface);
            ui.rounded(Rect::new(area.x + 14, row_y + 1, short.min(area.width / 3), 1), t.hover, t.surface);
        }
        return;
    }
    if tracks.is_empty() {
        if search.has_any() {
            let heading = format(&app.dict.search.no_results, &[("query", &search.results_for)]);
            let y = area.y + area.height / 3;
            centered_title(ui, area, y, &heading);
            paragraph(ui, Rect::new(area.x, y + 2, area.width, 2), &app.dict.search.no_results_hint, true);
        }
        return;
    }

    let mut first = 0;
    if search.filter() == Filter::All {
        y += 1;
        title(ui, area.x, y, area.width, &app.dict.search.top_result);
        top_result(ui, Rect::new(area.x, y + 2, area.width.min(72), 7), &tracks[0]);
        y += 11;
        first = 1;
    }
    title(ui, area.x, y, area.width, &app.dict.search.tracks);
    let list = Rect::new(area.x, y + 2, area.width, area.bottom().saturating_sub(y + 2));
    track_list(ui, list, &tracks, first, Source::Search, None, false);
}

fn top_result(ui: &mut Ui, area: Rect, track: &Track) {
    let app = ui.app;
    let t = ui.theme();
    let bg = if app.hovered(area) { t.hover } else { t.raised };
    ui.rounded(area, bg, t.surface);
    ui.hits.add(Hit {
        area,
        double: Some(Action::Play(Source::Search, 0)),
        context: Some(Action::TrackMenu(Source::Search, 0)),
        ..Hit::default()
    });
    ui.cover(Rect::new(area.x + 2, area.y + 1, 10, 5), track.artwork.as_deref(), Fallback::Notes, bg);
    let x = area.x + 15;
    let width = area.right().saturating_sub(x + BUTTON_WIDTH + 4);
    ui.text(x, area.y + 2, width, &track.title, Style::new().fg(t.text).bg(bg).add_modifier(Modifier::BOLD));
    ui.text(x, area.y + 3, width, &track.artists.join(", "), Style::new().fg(t.muted).bg(bg));
    ui.badge(x, area.y + 4, width, track.provider, bg);
    let playing = app.status == Status::Playing && app.is_current(track);
    let icon = if playing { Icon::Pause } else { Icon::Play };
    let button = Rect::new(area.right() - BUTTON_WIDTH - 3, area.y + 3, BUTTON_WIDTH, 2);
    ui.round_button(button, icon, t.accent, t.on_accent, bg, Action::Play(Source::Search, 0));
}

/// Rows like the window's track list; `first` skips rows shown elsewhere (the top result).
fn track_list(ui: &mut Ui, area: Rect, tracks: &[Track], first: usize, source: Source, playlist_id: Option<&str>, show_header: bool) {
    let app = ui.app;
    let t = ui.theme();
    let show_album = show_header && area.width >= ALBUM_MIN_WIDTH && tracks.iter().any(|track| track.album.is_some());
    let duration_x = area.right().saturating_sub(7);
    let actions_x = duration_x.saturating_sub(BUTTON_WIDTH * 2 + 1);
    let album_width = if show_album { area.width / 4 } else { 0 };
    let album_x = actions_x.saturating_sub(album_width + 2);
    let main_x = area.x + 6 + ROW_COVER + 2;
    let main_width = album_x.saturating_sub(main_x + 2);

    let mut list = area;
    if show_header {
        let style = Style::new().fg(t.faint).bg(t.surface);
        ui.text(area.x + 3, area.y, 2, "#", style);
        ui.text(main_x, area.y, main_width, &app.dict.track.title, style);
        if show_album {
            ui.text(album_x, area.y, album_width, &app.dict.track.album, style);
        }
        ui.glyph(duration_x + 3, area.y, Icon::Clock, t.faint, t.surface);
        ui.fill(Rect::new(area.x, area.y + 1, area.width, 1), t.surface);
        let rule = "─".repeat(area.width as usize);
        ui.text(area.x, area.y + 1, area.width, &rule, Style::new().fg(t.line).bg(t.surface));
        list = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(2));
    }

    let rows = tracks.len().saturating_sub(first);
    let visible = (list.height / ROW_HEIGHT) as usize;
    let max_offset = rows.saturating_sub(visible);
    ui.hits.scroll(list, ScrollId::Main, max_offset);
    let offset = app.offset(ScrollId::Main).min(max_offset);
    let current = app.queue.current().map(key_of);

    for (slot, index) in (first + offset..tracks.len()).take(visible).enumerate() {
        let track = &tracks[index];
        let row = Rect::new(list.x, list.y + slot as u16 * ROW_HEIGHT, list.width, ROW_HEIGHT);
        let hovered = app.hovered(row);
        let active = current.as_ref() == Some(&key_of(track));
        let playing = active && app.status == Status::Playing;
        let bg = if hovered { t.hover } else { t.surface };
        if hovered {
            ui.rounded(row, bg, t.surface);
        }
        ui.hits.add(Hit {
            area: row,
            double: Some(Action::Play(source.clone(), index)),
            context: Some(Action::TrackMenu(source.clone(), index)),
            drag: playlist_id.map(|id| Drag::PlaylistTrack(id.to_owned(), index)),
            ..Hit::default()
        });

        let middle = row.y + 1;
        if hovered {
            let icon = if playing { Icon::Pause } else { Icon::Play };
            ui.icon_button(row.x + 1, middle, icon, t.text, bg, Action::Play(source.clone(), index));
        } else if active {
            ui.bars(row.x + 2, middle, playing, bg);
        } else {
            let number = (index + 1).to_string();
            let x = row.x + 4u16.saturating_sub(display_width(&number) as u16);
            ui.text(x, middle, 4, &number, Style::new().fg(t.faint).bg(bg));
        }

        ui.cover(Rect::new(row.x + 6, row.y, ROW_COVER, ROW_HEIGHT), track.artwork.as_deref(), Fallback::Notes, bg);
        let title_style = if active { Style::new().fg(t.accent).bg(bg) } else { Style::new().fg(t.text).bg(bg) };
        ui.text(main_x, row.y, main_width, &track.title, title_style);
        ui.text(main_x, row.y + 1, main_width, &track.artists.join(", "), Style::new().fg(t.muted).bg(bg));
        ui.badge(main_x, row.y + 2, main_width, track.provider, bg);
        if show_album {
            let album = track.album.as_deref().unwrap_or_default();
            ui.text(album_x, middle, album_width, album, Style::new().fg(t.muted).bg(bg));
        }

        let liked = app.is_liked(track);
        if hovered {
            ui.icon_button(actions_x, middle, Icon::Enqueue, t.muted, bg, Action::EnqueueFrom(source.clone(), index));
        }
        if hovered || liked {
            let (icon, color) = if liked { (Icon::HeartFill, t.accent) } else { (Icon::Heart, t.muted) };
            ui.icon_button(actions_x + BUTTON_WIDTH, middle, icon, color, bg, Action::ToggleLikeFrom(source.clone(), index));
        }
        let duration = clock(track.duration_ms);
        let x = duration_x + 6u16.saturating_sub(display_width(&duration) as u16);
        ui.text(x, middle, 6, &duration, Style::new().fg(t.muted).bg(bg));
    }
}

/// Liked tracks (`None`) or a saved playlist or album.
fn collection(ui: &mut Ui, area: Rect, saved: Option<&SavedCollection>) {
    let app = ui.app;
    let t = ui.theme();
    let dict = &app.dict;
    let (source, tracks): (Source, &[Track]) = match saved {
        Some(c) => (Source::Collection(c.id.clone()), &c.tracks),
        None => (Source::Liked, &app.library.liked),
    };
    let is_user = saved.is_some_and(|c| c.origin == Origin::User);

    let cover = Rect::new(area.x, area.y + 1, HERO_COVER_WIDTH, HERO_COVER_HEIGHT);
    match saved {
        Some(c) => ui.cover(cover, cover_of(c), Fallback::Notes, t.surface),
        None => ui.cover(cover, None, Fallback::Liked, t.surface),
    }
    let x = cover.right() + 3;
    let width = area.right().saturating_sub(x);
    let kind = match saved.map(|c| c.kind) {
        Some(CollectionKind::Album) => &dict.collection.album,
        _ => &dict.collection.playlist,
    };
    ui.text(x, area.y + 3, width, kind, Style::new().fg(t.muted).bg(t.surface));
    let name = saved.map_or(dict.nav.liked.as_str(), |c| c.name.as_str());
    let name_style = Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::BOLD);
    match saved {
        Some(c) if is_user => {
            let shown = ellipsize(name, width.saturating_sub(ICON_WIDTH + 1) as usize);
            let name_area = Rect::new(x, area.y + 5, display_width(&shown) as u16 + ICON_WIDTH + 1, 1);
            let hovered = app.hovered(name_area);
            let style = if hovered { name_style.add_modifier(Modifier::UNDERLINED) } else { name_style };
            let used = ui.text(x, area.y + 5, width, &shown, style);
            if hovered {
                ui.glyph(x + used + 1, area.y + 5, Icon::Pencil, t.muted, t.surface);
            }
            ui.hits.click(name_area, Action::StartRename(c.id.clone()));
        }
        _ => {
            ui.text(x, area.y + 5, width, name, name_style);
        }
    }
    let mut meta_x = x;
    if let Some(owner) = saved.and_then(|c| c.owner.as_deref()) {
        meta_x += ui.text(meta_x, area.y + 7, width, owner, Style::new().fg(t.text).bg(t.surface));
        meta_x += ui.text(meta_x, area.y + 7, 5, "  ·  ", Style::new().fg(t.faint).bg(t.surface));
    }
    ui.text(meta_x, area.y + 7, area.right().saturating_sub(meta_x), &dict.tracks(tracks.len()), Style::new().fg(t.muted).bg(t.surface));
    if let Some(provider) = saved.and_then(|c| c.provider) {
        let used = ui.text(x, area.y + 9, width, &dict.collection.imported_from, Style::new().fg(t.faint).bg(t.surface));
        ui.badge(x + used + 1, area.y + 9, width.saturating_sub(used + 1), provider, t.surface);
    }

    let actions_y = cover.bottom() + 1;
    let mut actions_x = area.x;
    if !tracks.is_empty() {
        let icon = if app.is_playing_from(&source) { Icon::Pause } else { Icon::Play };
        ui.round_button(Rect::new(actions_x, actions_y, 6, 3), icon, t.accent, t.on_accent, t.surface, Action::PlayAll(source.clone()));
        actions_x += 8;
        let shuffle = if app.queue.shuffle { t.accent } else { t.muted };
        ui.round_button(Rect::new(actions_x, actions_y, 4, 3), Icon::Shuffle, t.surface, shuffle, t.surface, Action::ShufflePlay(source.clone()));
        actions_x += 6;
    }
    if let Some(c) = saved {
        ui.round_button(Rect::new(actions_x, actions_y, 4, 3), Icon::More, t.surface, t.muted, t.surface, Action::CollectionMenu(c.id.clone()));
    }

    let list = Rect::new(area.x, actions_y + 4, area.width, area.bottom().saturating_sub(actions_y + 4));
    if tracks.is_empty() {
        let (heading, body) = match saved {
            None => (dict.collection.liked_empty_title.as_str(), Some(dict.collection.liked_empty_body.as_str())),
            Some(_) if is_user => (dict.playlist.empty_title.as_str(), Some(dict.playlist.empty_body.as_str())),
            Some(_) => (dict.collection.empty.as_str(), None),
        };
        title(ui, list.x, list.y + 1, list.width, heading);
        if let Some(body) = body {
            paragraph(ui, Rect::new(list.x, list.y + 3, list.width, 2), body, false);
        }
        return;
    }
    let playlist_id = saved.filter(|_| is_user).map(|c| c.id.as_str());
    track_list(ui, list, tracks, 0, source, playlist_id, true);
}

fn settings(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    let t = ui.theme();
    let dict = &app.dict.settings;
    let prefs = app.prefs();
    let max_offset = (SETTINGS_HEIGHT - area.height as i32).max(0) as usize;
    ui.hits.scroll(area, ScrollId::Main, max_offset);
    let offset = app.offset(ScrollId::Main).min(max_offset) as i32;

    // Rows scroll by `offset`; anything outside the page is skipped.
    let row = |line: i32| -> Option<u16> {
        let y = area.y as i32 + line - offset;
        (y >= area.y as i32 && y < area.bottom() as i32).then_some(y as u16)
    };
    let label_style = Style::new().fg(t.text).bg(t.surface);
    let hint_style = Style::new().fg(t.muted).bg(t.surface);
    let field_width = area.width.min(SETTINGS_WIDTH);
    let right = area.x + field_width;
    let text_width = field_width.saturating_sub(CONTROL_COLUMN);

    let mut line = 0;
    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &dict.title);
    }
    line += 3;
    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &dict.language);
    }
    if let Some(y) = row(line + 2) {
        let options: Vec<(&'static str, String)> = LOCALES.iter().map(|(id, name)| (*id, (*name).to_owned())).collect();
        ui.segmented(area.x, y, &options, app.dict.locale(), t.surface, Action::SetLocale);
    }
    line += 5;
    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &dict.appearance);
    }
    line += 2;

    if let Some(y) = row(line) {
        ui.text(area.x, y, text_width, &dict.theme, label_style);
        let options = [
            (ThemeChoice::Dark, dict.theme_dark.clone()),
            (ThemeChoice::Light, dict.theme_light.clone()),
            (ThemeChoice::System, dict.theme_system.clone()),
        ];
        let width: u16 = options.iter().map(|(_, label)| segment_width(label)).sum();
        ui.segmented(right.saturating_sub(width), y, &options, prefs.theme, t.surface, Action::SetTheme);
    }
    line += 2;

    if let Some(y) = row(line) {
        ui.text(area.x, y, text_width, &dict.accent, label_style);
        let mut x = right.saturating_sub(ACCENTS.len() as u16 * SWATCH_STEP - 1);
        for (id, hex) in ACCENTS {
            let color = parse_hex(hex).unwrap_or(t.accent);
            let swatch = Rect::new(x, y, ICON_WIDTH, 1);
            let visual = Visual::Swatch {
                color: paint::rgb(color),
                chosen: prefs.accent == id,
                ring: paint::rgb(t.text),
                check: paint::rgb(t.on_accent),
                bg: paint::rgb(t.surface),
            };
            if !ui.picture(swatch, visual) {
                let mark = if prefs.accent == id { "✓ " } else { "  " };
                ui.text(x, y, ICON_WIDTH, mark, Style::new().fg(t.on_accent).bg(color));
            }
            ui.hits.click(swatch, Action::SetAccent(id));
            x += SWATCH_STEP;
        }
    }
    line += 2;

    let field = |ui: &mut Ui, line: i32, label: &str, hint: &str| {
        if let Some(y) = row(line) {
            ui.text(area.x, y, text_width, label, label_style);
        }
        if let Some(y) = row(line + 1) {
            ui.text(area.x, y, text_width, hint, hint_style);
        }
        row(line)
    };
    if let Some(y) = field(ui, line, &dict.motion, &dict.motion_hint) {
        ui.switch(right.saturating_sub(4), y, prefs.animations, t.surface, Action::ToggleAnimations);
    }
    line += 3;
    if let Some(y) = field(ui, line, &dict.output, &dict.output_hint) {
        let label = ellipsize(&app.output_label(), CONTROL_COLUMN as usize - 8);
        let width = display_width(&label) as u16 + ICON_WIDTH + 5;
        let select = Rect::new(right.saturating_sub(width), y, width, 1);
        let fill = if app.hovered(select) { t.hover } else { t.active };
        ui.rounded(select, fill, t.surface);
        ui.text(select.x + 2, y, width - ICON_WIDTH - 4, &label, Style::new().fg(t.text).bg(fill));
        ui.glyph(select.right() - ICON_WIDTH - 1, y, Icon::Down, t.muted, fill);
        ui.hits.click(select, Action::DeviceMenu);
    }
    line += 3;
    if let Some(y) = field(ui, line, &dict.discord, &dict.discord_hint) {
        ui.switch(right.saturating_sub(4), y, prefs.discord_presence, t.surface, Action::ToggleDiscord);
    }
    line += 4;

    let tui = &app.dict.tui;
    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &tui.window_title);
    }
    if let Some(y) = row(line + 2) {
        ui.text(area.x, y, text_width, &tui.window_hint, hint_style);
        let width = pill_width(Some(Icon::Window), &tui.window_action);
        ui.pill(right.saturating_sub(width), y, Some(Icon::Window), &tui.window_action, PillStyle::Secondary, t.surface, Action::SwitchToWindow);
    }
    line += 5;

    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &dict.about);
    }
    if let Some(y) = row(line + 2) {
        let product = format!("SCARPIFY {}", env!("CARGO_PKG_VERSION"));
        ui.text(area.x, y, area.width, &product, Style::new().fg(t.text).bg(t.surface).add_modifier(Modifier::BOLD));
    }
    if let Some(y) = row(line + 3) {
        ui.text(area.x, y, field_width, &dict.about_body, hint_style);
    }
    if let Some(y) = row(line + 5) {
        ui.pill(area.x, y, Some(Icon::External), &dict.source_code, PillStyle::Secondary, t.surface, Action::OpenUrl(REPOSITORY_URL.into()));
    }
}

