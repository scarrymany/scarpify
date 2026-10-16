//! The page in the middle: home, search, a playlist or album, liked tracks, settings.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::chrome::cover_of;
use super::{clock, display_width, ellipsize, Fallback, Ui};
use crate::library::{Origin, SavedCollection};
use crate::model::{CollectionKind, Provider, Track};
use crate::mode::{Theme as ThemeChoice, ACCENTS};
use crate::tui::app::{Action, Drag, Filter, Hit, Route, ScrollId, Source, Status, REPOSITORY_URL};
use crate::tui::i18n::{format, LOCALES};
use crate::tui::queue::key_of;
use crate::tui::theme::{local_hour, parse_hex, provider_color};

const PAD_X: u16 = 3;
const CARD_WIDTH: u16 = 22;
const CARD_GAP: u16 = 2;
const CARD_ART_HEIGHT: u16 = 10;
const CARD_HEIGHT: u16 = CARD_ART_HEIGHT + 5;
const ROW_STEP: u16 = 2;
const HERO_ART_WIDTH: u16 = 24;
const HERO_ART_HEIGHT: u16 = 12;
const SKELETON_ROWS: usize = 8;
const ALBUM_MIN_WIDTH: u16 = 90;
const SETTINGS_HEIGHT: i32 = 42;
const SETTINGS_WIDTH: u16 = 96;
const CONTROL_COLUMN: u16 = 30;

pub fn main(ui: &mut Ui, area: Rect) {
    let app = ui.app;
    ui.fill(area, ui.theme().surface);
    let inner = Rect::new(area.x + PAD_X, area.y + 1, area.width.saturating_sub(PAD_X * 2), area.height.saturating_sub(1));
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
    ui.text(x, y, width, text, Style::new().fg(t.text).add_modifier(Modifier::BOLD));
}

fn centered_title(ui: &mut Ui, area: Rect, y: u16, text: &str) {
    let x = area.x + area.width.saturating_sub(display_width(text) as u16) / 2;
    title(ui, x, y, area.width, text);
}

fn paragraph(ui: &mut Ui, area: Rect, text: &str, centered: bool) {
    let t = ui.theme();
    let mut widget = Paragraph::new(text.to_owned()).style(Style::new().fg(t.muted)).wrap(Wrap { trim: true });
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
        let search = format!("{}  {}", t.icons.search, app.dict.home.search_action);
        let import = &app.dict.importer.action;
        let total = display_width(&search) as u16 + display_width(import) as u16 + 10;
        let x = area.x + area.width.saturating_sub(total) / 2;
        let used = ui.pill(x, y + 5, &search, true, Action::FocusSearch);
        ui.pill(x + used + 2, y + 5, import, false, Action::OpenImport);
        return;
    }

    let mut y = area.y + 4;
    if !library.recent.is_empty() {
        let cards = library
            .recent
            .iter()
            .enumerate()
            .map(|(index, track)| Card {
                art: track.artwork.as_deref(),
                fallback: Fallback::Note,
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
        art: None,
        fallback: Fallback::Liked,
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
            art: cover_of(collection),
            fallback: Fallback::Note,
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
    if height > 3 {
        shelf(ui, Rect::new(area.x, y, area.width, height), 1, &app.dict.home.playlists, cards);
    }
}

struct Card<'a> {
    art: Option<&'a str>,
    fallback: Fallback,
    title: &'a str,
    subtitle: String,
    provider: Option<Provider>,
    playing: bool,
    /// Collections open on click; tracks play.
    open: Option<Action>,
    play: Action,
    context: Option<Action>,
}

/// A titled row of cards that scrolls sideways with the wheel or the arrows.
fn shelf(ui: &mut Ui, area: Rect, index: usize, heading: &str, cards: Vec<Card>) {
    let id = ScrollId::Shelf(index);
    let visible = ((area.width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).max(1) as usize;
    let max_offset = cards.len().saturating_sub(visible);
    let offset = ui.app.offset(id).min(max_offset);
    title(ui, area.x, area.y, area.width.saturating_sub(10), heading);
    if max_offset > 0 {
        let x = area.right().saturating_sub(8);
        let step = visible as isize;
        if offset > 0 {
            ui.icon(x, area.y, "‹", false, Action::Scroll(id, -step, max_offset));
        }
        if offset < max_offset {
            ui.icon(x + 4, area.y, "›", false, Action::Scroll(id, step, max_offset));
        }
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
    ui.fill(area, bg);
    let art_height = CARD_ART_HEIGHT.min(area.height.saturating_sub(4));
    ui.art(Rect::new(area.x + 1, area.y + 1, area.width - 2, art_height), card.art, card.fallback);
    ui.hits.add(Hit {
        area,
        click: Some(card.open.clone().unwrap_or_else(|| card.play.clone())),
        context: card.context,
        ..Hit::default()
    });

    let text_y = area.y + art_height + 2;
    let width = area.width - 2;
    // The play button sits beside the title: drawing over a picture would cut a hole in it.
    let show_play = hovered || card.playing;
    let title_width = if show_play { width.saturating_sub(6) } else { width };
    ui.text(area.x + 1, text_y, title_width, card.title, Style::new().fg(t.text).bg(bg).add_modifier(Modifier::BOLD));
    ui.text(area.x + 1, text_y + 1, width, &card.subtitle, Style::new().fg(t.muted).bg(bg));
    if let Some(provider) = card.provider {
        ui.badge(area.x + 1, text_y + 2, width, provider);
    }
    if show_play {
        let glyph = if card.playing { "⏸" } else { "▶" };
        let button = Rect::new(area.right() - 6, text_y, 5, 1);
        let style = Style::new().fg(t.on_accent).bg(t.accent).add_modifier(Modifier::BOLD);
        ui.button(button, Line::from(format!("  {glyph}  ")), style, style, card.play);
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
        let names = [Provider::Youtube, Provider::Soundcloud].map(|p| display_width(app.dict.provider(p)) + 2);
        let x = area.x + area.width.saturating_sub((names[0] + names[1] + 4) as u16) / 2;
        let used = ui.badge(x, y + 5, area.width, Provider::Youtube);
        ui.badge(x + used + 4, y + 5, area.width, Provider::Soundcloud);
        return;
    }

    let filters = [
        (Filter::All, app.dict.search.all.as_str()),
        (Filter::Youtube, app.dict.provider(Provider::Youtube)),
        (Filter::Soundcloud, app.dict.provider(Provider::Soundcloud)),
    ];
    let mut x = area.x;
    for (filter, label) in filters {
        let width = display_width(label) as u16 + 4;
        let chip = Rect::new(x, area.y, width, 1);
        let (normal, hover) = if search.filter() == filter {
            let style = Style::new().fg(t.bg).bg(t.text).add_modifier(Modifier::BOLD);
            (style, style)
        } else {
            (Style::new().fg(t.text).bg(t.active), Style::new().fg(t.text).bg(t.hover))
        };
        ui.button(chip, Line::from(format!("  {label}  ")), normal, hover, Action::SetFilter(filter));
        x += width + 1;
    }

    let mut y = area.y + 2;
    for provider in search.shown_providers() {
        if let Some(error) = search.result(*provider).and_then(|r| r.error.as_deref()) {
            let text = format(&app.dict.search.provider_error, &[("provider", app.dict.provider(*provider)), ("error", error)]);
            ui.text(area.x, y, area.width, &format!("{}  {text}", t.icons.warning), Style::new().fg(t.danger));
            y += 1;
        }
    }

    let tracks = search.tracks();
    if tracks.is_empty() && search.waiting() {
        for row in 0..SKELETON_ROWS {
            let row_y = y + 1 + row as u16 * 3;
            if row_y + 2 > area.bottom() {
                break;
            }
            ui.fill(Rect::new(area.x + 6, row_y, 4, 2), t.raised);
            let long = 20 + (row * 17 % 35) as u16;
            let short = 12 + (row * 11 % 20) as u16;
            ui.fill(Rect::new(area.x + 12, row_y, long.min(area.width / 2), 1), t.raised);
            ui.fill(Rect::new(area.x + 12, row_y + 1, short.min(area.width / 3), 1), t.hover);
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
        top_result(ui, Rect::new(area.x, y + 2, area.width.min(72), 8), &tracks[0]);
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
    ui.fill(area, bg);
    ui.hits.add(Hit {
        area,
        double: Some(Action::Play(Source::Search, 0)),
        context: Some(Action::TrackMenu(Source::Search, 0)),
        ..Hit::default()
    });
    ui.art(Rect::new(area.x + 2, area.y + 1, 12, 6), track.artwork.as_deref(), Fallback::Note);
    let x = area.x + 16;
    let width = area.right().saturating_sub(x + 10);
    ui.text(x, area.y + 2, width, &track.title, Style::new().fg(t.text).bg(bg).add_modifier(Modifier::BOLD));
    ui.text(x, area.y + 3, width, &track.artists.join(", "), Style::new().fg(t.muted).bg(bg));
    ui.badge(x, area.y + 5, width, track.provider);
    let playing = app.status == Status::Playing && app.is_current(track);
    let glyph = if playing { "⏸" } else { "▶" };
    let button = Rect::new(area.right() - 8, area.y + 5, 6, 1);
    let style = Style::new().fg(t.on_accent).bg(t.accent).add_modifier(Modifier::BOLD);
    ui.button(button, Line::from(format!("  {glyph}   ")), style, style, Action::Play(Source::Search, 0));
}

/// Two-line rows like the window's track list; `first` skips rows shown elsewhere.
fn track_list(
    ui: &mut Ui,
    area: Rect,
    tracks: &[Track],
    first: usize,
    source: Source,
    playlist_id: Option<&str>,
    show_header: bool,
) {
    let app = ui.app;
    let t = ui.theme();
    let show_album = show_header && area.width >= ALBUM_MIN_WIDTH && tracks.iter().any(|track| track.album.is_some());
    let duration_x = area.right().saturating_sub(6);
    let actions_x = duration_x.saturating_sub(9);
    let album_width = if show_album { area.width / 4 } else { 0 };
    let album_x = actions_x.saturating_sub(album_width + 1);
    let main_x = area.x + 12;
    let main_width = album_x.saturating_sub(main_x + 1);

    let mut list = area;
    if show_header {
        let style = Style::new().fg(t.faint);
        ui.text(area.x + 3, area.y, 2, "#", style);
        ui.text(main_x, area.y, main_width, &app.dict.track.title, style);
        if show_album {
            ui.text(album_x, area.y, album_width, &app.dict.track.album, style);
        }
        ui.text(duration_x + 2, area.y, 3, "◷", style);
        ui.line(Rect::new(area.x, area.y + 1, area.width, 1), Line::styled("─".repeat(area.width as usize), Style::new().fg(t.line)));
        list = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(2));
    }

    let rows = tracks.len().saturating_sub(first);
    let visible = (list.height / ROW_STEP) as usize;
    let max_offset = rows.saturating_sub(visible);
    ui.hits.scroll(list, ScrollId::Main, max_offset);
    let offset = app.offset(ScrollId::Main).min(max_offset);
    let current = app.queue.current().map(key_of);

    for (slot, index) in (first + offset..tracks.len()).take(visible).enumerate() {
        let track = &tracks[index];
        let row = Rect::new(list.x, list.y + slot as u16 * ROW_STEP, list.width, ROW_STEP);
        let hovered = app.hovered(row);
        let active = current.as_ref() == Some(&key_of(track));
        let playing = active && app.status == Status::Playing;
        let bg = if hovered { t.hover } else { t.surface };
        ui.fill(row, bg);
        ui.hits.add(Hit {
            area: row,
            double: Some(Action::Play(source.clone(), index)),
            context: Some(Action::TrackMenu(source.clone(), index)),
            drag: playlist_id.map(|id| Drag::PlaylistTrack(id.to_owned(), index)),
            ..Hit::default()
        });

        if hovered {
            let glyph = if playing { "⏸" } else { "▶" };
            let style = Style::new().fg(t.text).bg(bg);
            ui.button(Rect::new(row.x, row.y, 5, 1), Line::from(format!("  {glyph}")), style, style, Action::Play(source.clone(), index));
        } else if active {
            let bars = ui.bars(playing);
            ui.text(row.x + 1, row.y, 4, &format!(" {bars}"), Style::new().fg(t.accent).bg(bg));
        } else {
            let number = (index + 1).to_string();
            let x = row.x + 4u16.saturating_sub(display_width(&number) as u16);
            ui.text(x, row.y, 4, &number, Style::new().fg(t.faint).bg(bg));
        }

        ui.art(Rect::new(row.x + 6, row.y, 4, 2), track.artwork.as_deref(), Fallback::Note);
        let title_style = if active { Style::new().fg(t.accent).bg(bg) } else { Style::new().fg(t.text).bg(bg) };
        ui.text(main_x, row.y, main_width, &track.title, title_style);
        let artists = Line::from(vec![
            Span::styled("● ", Style::new().fg(provider_color(track.provider)).bg(bg)),
            Span::styled(track.artists.join(", "), Style::new().fg(t.muted).bg(bg)),
        ]);
        ui.line(Rect::new(main_x, row.y + 1, main_width, 1), artists);
        if show_album {
            let album = track.album.as_deref().unwrap_or_default();
            ui.text(album_x, row.y, album_width, album, Style::new().fg(t.muted).bg(bg));
        }

        let liked = app.is_liked(track);
        if hovered {
            ui.icon(actions_x, row.y, "+", false, Action::EnqueueFrom(source.clone(), index));
        }
        if hovered || liked {
            let heart = if liked { "♥" } else { "♡" };
            ui.icon(actions_x + 4, row.y, heart, liked, Action::ToggleLikeFrom(source.clone(), index));
        }
        let duration = clock(track.duration_ms);
        let x = duration_x + 5u16.saturating_sub(display_width(&duration) as u16);
        ui.text(x, row.y, 5, &duration, Style::new().fg(t.muted).bg(bg));
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

    let cover = Rect::new(area.x, area.y + 1, HERO_ART_WIDTH, HERO_ART_HEIGHT);
    match saved {
        Some(c) => ui.art(cover, cover_of(c), Fallback::Note),
        None => ui.art(cover, None, Fallback::Liked),
    }
    let x = cover.right() + 3;
    let width = area.right().saturating_sub(x);
    let kind = match saved.map(|c| c.kind) {
        Some(CollectionKind::Album) => &dict.collection.album,
        _ => &dict.collection.playlist,
    };
    ui.text(x, area.y + 3, width, kind, Style::new().fg(t.muted));
    let name = saved.map_or(dict.nav.liked.as_str(), |c| c.name.as_str());
    let name_style = Style::new().fg(t.text).add_modifier(Modifier::BOLD);
    match saved {
        Some(c) if is_user => {
            let shown = ellipsize(name, width as usize);
            let name_area = Rect::new(x, area.y + 5, display_width(&shown) as u16, 1);
            let hover = name_style.add_modifier(Modifier::UNDERLINED);
            ui.button(name_area, Line::from(shown), name_style, hover, Action::StartRename(c.id.clone()));
        }
        _ => {
            ui.text(x, area.y + 5, width, name, name_style);
        }
    }
    let mut meta = Vec::new();
    if let Some(owner) = saved.and_then(|c| c.owner.as_deref()) {
        meta.push(Span::styled(format!("{owner}  ·  "), Style::new().fg(t.text)));
    }
    meta.push(Span::styled(dict.tracks(tracks.len()), Style::new().fg(t.muted)));
    ui.line(Rect::new(x, area.y + 7, width, 1), Line::from(meta));
    if let Some(provider) = saved.and_then(|c| c.provider) {
        let used = ui.text(x, area.y + 9, width, &dict.collection.imported_from, Style::new().fg(t.faint));
        ui.badge(x + used + 1, area.y + 9, width.saturating_sub(used + 1), provider);
    }

    let actions_y = cover.bottom() + 1;
    if !tracks.is_empty() {
        let glyph = if app.is_playing_from(&source) { "⏸" } else { "▶" };
        let play = Rect::new(area.x, actions_y, 7, 1);
        let style = Style::new().fg(t.on_accent).bg(t.accent).add_modifier(Modifier::BOLD);
        let hover = Style::new().fg(t.bg).bg(t.text).add_modifier(Modifier::BOLD);
        ui.button(play, Line::from(format!("   {glyph}   ")), style, hover, Action::PlayAll(source.clone()));
        ui.icon(area.x + 9, actions_y, "⇄", app.queue.shuffle, Action::ShufflePlay(source.clone()));
    }
    if let Some(c) = saved {
        ui.icon(area.x + 14, actions_y, "⋯", false, Action::CollectionMenu(c.id.clone()));
    }

    let list = Rect::new(area.x, actions_y + 2, area.width, area.bottom().saturating_sub(actions_y + 2));
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

    // Rows scroll by `offset`; anything outside the page is simply skipped.
    let row = |line: i32| -> Option<u16> {
        let y = area.y as i32 + line - offset;
        (y >= area.y as i32 && y < area.bottom() as i32).then_some(y as u16)
    };
    let label_style = Style::new().fg(t.text);
    let hint_style = Style::new().fg(t.muted);
    let field_width = area.width.min(SETTINGS_WIDTH);
    let right = area.x + field_width;
    let text_width = field_width.saturating_sub(CONTROL_COLUMN);

    let mut line = 0;
    if let Some(y) = row(line) {
        title(ui, area.x, y, area.width, &dict.title);
    }
    line += 3;
    if let Some(y) = row(line) {
        ui.heading(area.x, y, area.width, &dict.language);
    }
    if let Some(y) = row(line + 2) {
        let options: Vec<(&'static str, String)> = LOCALES.iter().map(|(id, name)| (*id, (*name).to_owned())).collect();
        ui.segmented(area.x, y, &options, app.dict.locale(), Action::SetLocale);
    }
    line += 5;
    if let Some(y) = row(line) {
        ui.heading(area.x, y, area.width, &dict.appearance);
    }
    line += 2;

    if let Some(y) = row(line) {
        ui.text(area.x, y, text_width, &dict.theme, label_style);
        let options = [
            (ThemeChoice::Dark, dict.theme_dark.clone()),
            (ThemeChoice::Light, dict.theme_light.clone()),
            (ThemeChoice::System, dict.theme_system.clone()),
        ];
        let width: u16 = options.iter().map(|(_, label)| display_width(label) as u16 + 4).sum();
        ui.segmented(right.saturating_sub(width), y, &options, prefs.theme, Action::SetTheme);
    }
    line += 2;

    if let Some(y) = row(line) {
        ui.text(area.x, y, text_width, &dict.accent, label_style);
        let mut x = right.saturating_sub(ACCENTS.len() as u16 * 5);
        for (id, hex) in ACCENTS {
            let color = parse_hex(hex).unwrap_or(t.accent);
            let mark = if prefs.accent == id { " ✓  " } else { "    " };
            let style = Style::new().fg(t.on_accent).bg(color).add_modifier(Modifier::BOLD);
            ui.button(Rect::new(x, y, 4, 1), Line::from(mark), style, style, Action::SetAccent(id));
            x += 5;
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
        ui.switch(right.saturating_sub(6), y, prefs.animations, Action::ToggleAnimations);
    }
    line += 3;
    if let Some(y) = field(ui, line, &dict.output, &dict.output_hint) {
        let label = format!(" {}  ▾ ", ellipsize(&app.output_label(), CONTROL_COLUMN as usize - 6));
        let width = display_width(&label) as u16;
        let select = Rect::new(right.saturating_sub(width), y, width, 1);
        let (normal, hover) = (Style::new().fg(t.text).bg(t.active), Style::new().fg(t.text).bg(t.hover));
        ui.button(select, Line::from(label), normal, hover, Action::DeviceMenu);
    }
    line += 3;
    if let Some(y) = field(ui, line, &dict.discord, &dict.discord_hint) {
        ui.switch(right.saturating_sub(6), y, prefs.discord_presence, Action::ToggleDiscord);
    }
    line += 4;

    let tui = &app.dict.tui;
    if let Some(y) = row(line) {
        ui.heading(area.x, y, area.width, &tui.window_title);
    }
    if let Some(y) = row(line + 2) {
        ui.text(area.x, y, text_width, &tui.window_hint, hint_style);
        let label = format!("{}  {}", t.icons.window, tui.window_action);
        let width = display_width(&label) as u16 + 4;
        ui.pill(right.saturating_sub(width), y, &label, false, Action::SwitchToWindow);
    }
    line += 5;

    if let Some(y) = row(line) {
        ui.heading(area.x, y, area.width, &dict.about);
    }
    if let Some(y) = row(line + 2) {
        let product = format!("SCARPIFY {}", env!("CARGO_PKG_VERSION"));
        ui.text(area.x, y, area.width, &product, Style::new().fg(t.text).add_modifier(Modifier::BOLD));
    }
    if let Some(y) = row(line + 3) {
        ui.text(area.x, y, field_width, &dict.about_body, hint_style);
    }
    if let Some(y) = row(line + 5) {
        let label = format!("{}  ↗", dict.source_code);
        ui.pill(area.x, y, &label, false, Action::OpenUrl(REPOSITORY_URL.into()));
    }
}
