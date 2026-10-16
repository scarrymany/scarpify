//! Terminal interface state and everything a click can do. The screen mirrors the window:
//! title bar, sidebar, the current page, the queue panel and the player bar.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::i18n::{format, Dict};
use super::queue::{key_of, Queue, TrackKey};
use super::theme::Theme;
use crate::audio::devices::{self, OutputDevice};
use crate::audio::engine::{AudioEngine, PlayerEvent};
use crate::error::{Error, Result};
use crate::library::{Library, Origin, SavedCollection, Snapshot};
use crate::mode::{AppDirs, Mode, Theme as ThemeChoice, UiPrefs};
use crate::model::{Provider, ProviderResult, Track};
use crate::presence::{NowPlaying, Presence};
use crate::sources::Sources;

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);
const TOAST_TTL: Duration = Duration::from_secs(4);
const MAX_TOASTS: usize = 3;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const WHEEL_STEP: usize = 3;
/// Like most players, "previous" restarts a track that has already played this long.
const RESTART_THRESHOLD_MS: u64 = 3_000;
const PREFETCH_TTL: Duration = Duration::from_secs(10 * 60);
const VOLUME_STEP: f32 = 0.05;
const SEEK_STEP_MS: i64 = 5_000;
pub const REPOSITORY_URL: &str = "https://github.com/scarrymany/scarpify";
pub const AUTHOR_URL: &str = "https://t.me/yeet17";

pub struct Services {
    pub sources: Sources,
    pub engine: Arc<AudioEngine>,
    pub library: Library,
    pub presence: Presence,
}

/// Results of background work, delivered to the UI loop.
pub enum Message {
    Player(PlayerEvent),
    Searched { id: u64, provider: Provider, result: ProviderResult },
    Imported(Result<SavedCollection>),
    Played,
    PlayFailed { id: u64, error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Search,
    Liked,
    Collection(String),
    Settings,
}

/// Browser-like history, as the window's back and forward buttons have.
struct History {
    stack: Vec<Route>,
    index: usize,
}

impl History {
    fn current(&self) -> &Route {
        &self.stack[self.index]
    }

    fn go(&mut self, route: Route) {
        if *self.current() == route {
            return;
        }
        self.stack.truncate(self.index + 1);
        self.stack.push(route);
        self.index = self.stack.len() - 1;
    }

    fn forget(&mut self, route: &Route) {
        let current = self.current().clone();
        self.stack.retain(|r| r != route);
        if self.stack.is_empty() {
            self.stack.push(Route::Home);
        }
        self.index = self.stack.iter().position(|r| *r == current).unwrap_or(self.stack.len() - 1);
    }
}

/// Where a track row comes from, so a click resolves the track when it happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Search,
    Recent,
    Liked,
    Collection(String),
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Youtube,
    Soundcloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollId {
    Sidebar,
    Main,
    Queue,
    Shelf(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slider {
    Seek,
    Volume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drag {
    Collection(usize),
    PlaylistTrack(String, usize),
    Upcoming(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Search,
    Dialog,
}

#[derive(Clone)]
pub enum Action {
    Go(Route),
    Back,
    Forward,
    FocusSearch,
    ClearSearch,
    SetFilter(Filter),
    Play(Source, usize),
    PlayAll(Source),
    ShufflePlay(Source),
    Enqueue(Track),
    EnqueueFrom(Source, usize),
    ToggleLike(Track),
    ToggleLikeFrom(Source, usize),
    TrackMenu(Source, usize),
    CollectionMenu(String),
    AddMenu,
    DeviceMenu,
    OpenSubmenu(usize),
    TogglePlay,
    Next,
    Previous,
    ToggleShuffle,
    CycleRepeat,
    ToggleMute,
    ToggleQueue,
    ShuffleUpcoming,
    ClearUpcoming,
    CreatePlaylist,
    OpenImport,
    StartRename(String),
    AskDelete(String),
    Delete(String),
    SetPinned(String, bool),
    ResetCover(String),
    AddToPlaylist(String, Track),
    AddToNewPlaylist(Track),
    RemoveFromPlaylist(String, Track),
    OpenUrl(String),
    SetLocale(&'static str),
    SetTheme(ThemeChoice),
    SetAccent(&'static str),
    ToggleAnimations,
    ToggleDiscord,
    SetDevice(Option<String>),
    SwitchToWindow,
    DialogSubmit,
    CloseOverlay,
    /// Clicking beside an open menu only closes it.
    CloseMenu,
    DismissToast(usize),
    /// Scrolls a region by whole items; `max` comes from the last frame's layout.
    Scroll(ScrollId, isize, usize),
}

pub enum Dialog {
    Import { link: String, busy: bool, error: Option<String> },
    Rename { id: String, name: String },
    Confirm { title: String, body: String, confirm: String, action: Box<Action> },
}

pub struct MenuItem {
    pub icon: &'static str,
    pub label: String,
    pub danger: bool,
    pub checked: bool,
    pub action: Option<Action>,
    pub submenu: Vec<MenuItem>,
}

impl MenuItem {
    fn new(icon: &'static str, label: impl Into<String>, action: Action) -> Self {
        Self { icon, label: label.into(), danger: false, checked: false, action: Some(action), submenu: Vec::new() }
    }
}

pub struct Menu {
    pub anchor: Position,
    pub items: Vec<MenuItem>,
    pub open_submenu: Option<usize>,
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    shown_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Loading,
    Playing,
    Paused,
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub filter: Option<Filter>,
    pub youtube: Option<ProviderResult>,
    pub soundcloud: Option<ProviderResult>,
    pub results_for: String,
    pub pending: [bool; 2],
    request_id: u64,
    due: Option<Instant>,
}

impl SearchState {
    pub fn filter(&self) -> Filter {
        self.filter.unwrap_or(Filter::All)
    }

    /// Alternates providers in "All" so both services show near the top.
    pub fn tracks(&self) -> Vec<Track> {
        let youtube = self.youtube.as_ref().map_or(&[][..], |r| &r.tracks);
        let soundcloud = self.soundcloud.as_ref().map_or(&[][..], |r| &r.tracks);
        match self.filter() {
            Filter::Youtube => youtube.to_vec(),
            Filter::Soundcloud => soundcloud.to_vec(),
            Filter::All => {
                let mut merged = Vec::with_capacity(youtube.len() + soundcloud.len());
                for i in 0..youtube.len().max(soundcloud.len()) {
                    merged.extend(youtube.get(i).cloned());
                    merged.extend(soundcloud.get(i).cloned());
                }
                merged
            }
        }
    }

    pub fn shown_providers(&self) -> &'static [Provider] {
        match self.filter() {
            Filter::All => &[Provider::Youtube, Provider::Soundcloud],
            Filter::Youtube => &[Provider::Youtube],
            Filter::Soundcloud => &[Provider::Soundcloud],
        }
    }

    pub fn result(&self, provider: Provider) -> Option<&ProviderResult> {
        match provider {
            Provider::Youtube => self.youtube.as_ref(),
            _ => self.soundcloud.as_ref(),
        }
    }

    pub fn waiting(&self) -> bool {
        self.shown_providers().iter().any(|p| self.pending[slot(*p)])
    }

    pub fn has_any(&self) -> bool {
        self.youtube.is_some() || self.soundcloud.is_some()
    }
}

fn slot(provider: Provider) -> usize {
    match provider {
        Provider::Youtube => 0,
        _ => 1,
    }
}

/// One clickable region, registered while drawing; the last registered wins, so overlays
/// drawn on top also take the clicks.
#[derive(Default)]
pub struct Hit {
    pub area: Rect,
    pub click: Option<Action>,
    pub double: Option<Action>,
    pub context: Option<Action>,
    pub scroll: Option<(ScrollId, usize)>,
    pub slider: Option<Slider>,
    pub drag: Option<Drag>,
    pub field: Option<Field>,
    /// Dialogs and menus: nothing underneath reacts, not even to the wheel.
    pub modal: bool,
}

#[derive(Default)]
pub struct Hits(Vec<Hit>);

impl Hits {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn add(&mut self, hit: Hit) {
        self.0.push(hit);
    }

    pub fn click(&mut self, area: Rect, action: Action) {
        self.add(Hit { area, click: Some(action), ..Hit::default() });
    }

    pub fn scroll(&mut self, area: Rect, id: ScrollId, max: usize) {
        self.add(Hit { area, scroll: Some((id, max)), ..Hit::default() });
    }

    /// Swallows clicks so nothing under a dialog or menu reacts.
    pub fn block(&mut self, area: Rect, click: Option<Action>) {
        self.add(Hit { area, click: click.clone(), context: click, modal: true, ..Hit::default() });
    }

    fn at(&self, position: Position) -> Option<&Hit> {
        self.0.iter().rev().find(|hit| hit.area.contains(position))
    }

    fn scroll_at(&self, position: Position) -> Option<(ScrollId, usize)> {
        for hit in self.0.iter().rev().filter(|h| h.area.contains(position)) {
            if hit.scroll.is_some() {
                return hit.scroll;
            }
            if hit.modal {
                return None;
            }
        }
        None
    }

    fn rect_of(&self, slider: Slider) -> Option<Rect> {
        self.0.iter().rev().find(|h| h.slider == Some(slider)).map(|h| h.area)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Quit,
    Window,
}

pub struct App {
    services: Arc<Services>,
    tx: Sender<Message>,
    dirs: Arc<AppDirs>,
    prefs: UiPrefs,
    rich_terminal: bool,
    pub dict: Dict,
    pub theme: Theme,

    history: History,
    pub search: SearchState,
    pub focus: Option<Field>,
    pub dialog: Option<Dialog>,
    pub menu: Option<Menu>,
    pub toasts: Vec<Toast>,

    pub library: Snapshot,
    liked: HashSet<TrackKey>,

    pub queue: Queue,
    pub status: Status,
    pub position_ms: u64,
    /// Where the seek bar is being dragged to, shown until the button is released.
    pub scrub_ms: Option<u64>,
    play_id: u64,
    engine_loaded: bool,
    prefetched: HashMap<TrackKey, Instant>,
    pub queue_open: bool,

    pub scroll: HashMap<ScrollId, usize>,
    pub devices: Vec<OutputDevice>,
    pub mouse: Option<Position>,
    pressed: Option<Drag>,
    sliding: Option<Slider>,
    last_click: Option<(Instant, Position)>,
    /// Images must be repainted from scratch after anything covered them.
    pub repaint: bool,
    pub exit: Option<Exit>,
}

impl App {
    pub fn new(
        services: Arc<Services>,
        tx: Sender<Message>,
        dirs: Arc<AppDirs>,
        prefs: UiPrefs,
        rich_terminal: bool,
    ) -> Result<Self> {
        let library = services.library.snapshot()?;
        let mut app = Self {
            dict: Dict::load(&prefs.locale),
            theme: Theme::new(&prefs, rich_terminal),
            services,
            tx,
            dirs,
            prefs,
            rich_terminal,
            history: History { stack: vec![Route::Home], index: 0 },
            search: SearchState::default(),
            focus: None,
            dialog: None,
            menu: None,
            toasts: Vec::new(),
            library,
            liked: HashSet::new(),
            queue: Queue::default(),
            status: Status::Idle,
            position_ms: 0,
            scrub_ms: None,
            play_id: 0,
            engine_loaded: false,
            prefetched: HashMap::new(),
            queue_open: false,
            scroll: HashMap::new(),
            devices: Vec::new(),
            mouse: None,
            pressed: None,
            sliding: None,
            last_click: None,
            repaint: true,
            exit: None,
        };
        app.index_liked();
        Ok(app)
    }

    pub fn prefs(&self) -> &UiPrefs {
        &self.prefs
    }

    pub fn route(&self) -> &Route {
        self.history.current()
    }

    pub fn can_back(&self) -> bool {
        self.history.index > 0
    }

    pub fn can_forward(&self) -> bool {
        self.history.index + 1 < self.history.stack.len()
    }

    pub fn is_liked(&self, track: &Track) -> bool {
        self.liked.contains(&key_of(track))
    }

    pub fn is_current(&self, track: &Track) -> bool {
        self.queue.current().is_some_and(|current| key_of(current) == key_of(track))
    }

    pub fn volume(&self) -> f32 {
        self.prefs.volume
    }

    pub fn muted(&self) -> bool {
        self.prefs.muted || self.prefs.volume == 0.0
    }

    pub fn duration_ms(&self) -> u64 {
        self.queue.current().and_then(|t| t.duration_ms).unwrap_or(0)
    }

    pub fn collection(&self, id: &str) -> Option<&SavedCollection> {
        self.library.collections.iter().find(|c| c.id == id)
    }

    pub fn offset(&self, id: ScrollId) -> usize {
        self.scroll.get(&id).copied().unwrap_or(0)
    }

    pub fn hovered(&self, area: Rect) -> bool {
        self.overlay_free() && self.mouse.is_some_and(|m| area.contains(m))
    }

    /// Hover effects stay off under menus and dialogs, as in the window.
    fn overlay_free(&self) -> bool {
        self.menu.is_none() && self.dialog.is_none()
    }

    pub fn tracks_of(&self, source: &Source) -> Vec<Track> {
        match source {
            Source::Search => self.search.tracks(),
            Source::Recent => self.library.recent.clone(),
            Source::Liked => self.library.liked.clone(),
            Source::Collection(id) => self.collection(id).map(|c| c.tracks.clone()).unwrap_or_default(),
            Source::Queue => self.queue.tracks().to_vec(),
        }
    }

    fn track_at(&self, source: &Source, index: usize) -> Option<Track> {
        match source {
            Source::Search => self.search.tracks().into_iter().nth(index),
            Source::Recent => self.library.recent.get(index).cloned(),
            Source::Liked => self.library.liked.get(index).cloned(),
            Source::Collection(id) => self.collection(id)?.tracks.get(index).cloned(),
            Source::Queue => self.queue.tracks().get(index).cloned(),
        }
    }

    pub fn toast(&mut self, text: String, error: bool) {
        self.toasts.push(Toast { text, error, shown_at: Instant::now() });
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
        }
    }

    fn fail(&mut self, error: impl ToString) {
        let text = format(&self.dict.errors.generic, &[("error", &error.to_string())]);
        self.toast(text, true);
    }

    /// Timers: expiring toasts and the search debounce. Returns whether the screen changed.
    pub fn tick(&mut self) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|toast| toast.shown_at.elapsed() < TOAST_TTL);
        let mut changed = self.toasts.len() != before;
        if self.search.due.is_some_and(|due| Instant::now() >= due) {
            self.run_search();
            changed = true;
        }
        changed
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.exit = Some(Exit::Quit);
            return;
        }
        if let Some(field) = self.focus {
            self.on_text_key(field, key);
            return;
        }
        match key.code {
            KeyCode::Esc => self.dispatch(Action::CloseOverlay),
            KeyCode::Enter if self.dialog.is_some() => self.dispatch(Action::DialogSubmit),
            KeyCode::Char(' ') => self.dispatch(Action::TogglePlay),
            KeyCode::Char('f') | KeyCode::Char('l') if ctrl => self.dispatch(Action::FocusSearch),
            KeyCode::Right if ctrl => self.dispatch(Action::Next),
            KeyCode::Left if ctrl => self.dispatch(Action::Previous),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => self.seek_by(SEEK_STEP_MS),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => self.seek_by(-SEEK_STEP_MS),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => self.dispatch(Action::Back),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => self.dispatch(Action::Forward),
            KeyCode::Up if ctrl => self.set_volume(self.prefs.volume + VOLUME_STEP),
            KeyCode::Down if ctrl => self.set_volume(self.prefs.volume - VOLUME_STEP),
            _ => {}
        }
    }

    fn on_text_key(&mut self, field: Field, key: KeyEvent) {
        let text = match field {
            Field::Search => &mut self.search.query,
            Field::Dialog => match &mut self.dialog {
                Some(Dialog::Import { link, .. }) => link,
                Some(Dialog::Rename { name, .. }) => name,
                _ => {
                    self.focus = None;
                    return;
                }
            },
        };
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => text.push(c),
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Enter => match field {
                Field::Search => self.run_search(),
                Field::Dialog => self.dispatch(Action::DialogSubmit),
            },
            KeyCode::Esc => {
                self.focus = None;
                if field == Field::Dialog {
                    self.dispatch(Action::CloseOverlay);
                }
                return;
            }
            _ => return,
        }
        if field == Field::Search && matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace) {
            self.query_changed();
        }
    }

    /// Pasted text arrives in one piece (bracketed paste), e.g. a playlist link.
    pub fn on_paste(&mut self, pasted: &str) {
        let clean: String = pasted.chars().filter(|c| !c.is_control()).collect();
        match (self.focus, &mut self.dialog) {
            (Some(Field::Dialog) | None, Some(Dialog::Import { link, .. })) => link.push_str(&clean),
            (Some(Field::Dialog), Some(Dialog::Rename { name, .. })) => name.push_str(&clean),
            _ => {
                self.search.query.push_str(&clean);
                self.focus = Some(Field::Search);
                self.history.go(Route::Search);
                self.query_changed();
            }
        }
    }

    pub fn on_mouse(&mut self, event: MouseEvent, hits: &Hits) {
        let position = Position::new(event.column, event.row);
        self.mouse = Some(position);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => self.on_press(position, hits),
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(slider) = self.sliding {
                    self.slide(slider, position, hits, false);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => self.on_release(position, hits),
            MouseEventKind::Down(MouseButton::Right) => {
                if let Some(action) = hits.at(position).and_then(|h| h.context.clone()) {
                    self.dispatch(action);
                }
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                if let Some((id, max)) = hits.scroll_at(position) {
                    let offset = self.offset(id);
                    let next = if event.kind == MouseEventKind::ScrollDown {
                        (offset + WHEEL_STEP).min(max)
                    } else {
                        offset.saturating_sub(WHEEL_STEP)
                    };
                    self.scroll.insert(id, next);
                }
            }
            MouseEventKind::Moved => self.hover_menu(position, hits),
            _ => {}
        }
    }

    fn on_press(&mut self, position: Position, hits: &Hits) {
        let Some(hit) = hits.at(position) else {
            self.focus = None;
            return;
        };
        self.focus = hit.field;
        if let Some(slider) = hit.slider {
            self.sliding = Some(slider);
            self.slide(slider, position, hits, false);
            return;
        }
        self.pressed = hit.drag.clone();

        let double = self
            .last_click
            .is_some_and(|(at, last)| at.elapsed() < DOUBLE_CLICK && last == position);
        self.last_click = Some((Instant::now(), position));
        let action = match (&hit.double, double) {
            (Some(action), true) => Some(action.clone()),
            _ => hit.click.clone(),
        };
        if let Some(action) = action {
            self.dispatch(action);
        }
    }

    fn on_release(&mut self, position: Position, hits: &Hits) {
        if let Some(slider) = self.sliding.take() {
            self.slide(slider, position, hits, true);
            return;
        }
        let Some(source) = self.pressed.take() else { return };
        let Some(target) = hits.at(position).and_then(|h| h.drag.clone()) else { return };
        match (source, target) {
            (Drag::Collection(from), Drag::Collection(to)) if from != to => self.move_collection(from, to),
            (Drag::PlaylistTrack(id, from), Drag::PlaylistTrack(target_id, to)) if id == target_id && from != to => {
                match self.services.library.move_track(&id, from, to) {
                    Ok(_) => self.reload_library(),
                    Err(err) => self.fail(err),
                }
            }
            (Drag::Upcoming(from), Drag::Upcoming(to)) if from != to => self.queue.move_upcoming(from, to),
            _ => {}
        }
    }

    /// Dragging the seek bar only previews; the jump happens on release, like the window.
    fn slide(&mut self, slider: Slider, position: Position, hits: &Hits, release: bool) {
        let Some(area) = hits.rect_of(slider) else { return };
        let width = area.width.saturating_sub(1).max(1) as f64;
        let ratio = (position.x.saturating_sub(area.x) as f64 / width).clamp(0.0, 1.0);
        match slider {
            Slider::Volume => self.set_volume(ratio as f32),
            Slider::Seek => {
                let target = (self.duration_ms() as f64 * ratio) as u64;
                if release {
                    self.scrub_ms = None;
                    self.seek_to(target);
                } else {
                    self.scrub_ms = Some(target);
                }
            }
        }
    }

    fn hover_menu(&mut self, position: Position, hits: &Hits) {
        let Some(Action::OpenSubmenu(index)) = hits.at(position).and_then(|h| h.click.clone()) else { return };
        if let Some(menu) = &mut self.menu {
            menu.open_submenu = Some(index);
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        let closes_menu = !matches!(action, Action::OpenSubmenu(_));
        if closes_menu && self.menu.take().is_some() {
            self.repaint = true;
        }
        match action {
            Action::Go(route) => self.go(route),
            Action::Back => {
                if self.can_back() {
                    self.history.index -= 1;
                    self.scroll.remove(&ScrollId::Main);
                }
            }
            Action::Forward => {
                if self.can_forward() {
                    self.history.index += 1;
                    self.scroll.remove(&ScrollId::Main);
                }
            }
            Action::FocusSearch => {
                self.go(Route::Search);
                self.focus = Some(Field::Search);
            }
            Action::ClearSearch => {
                self.search.query.clear();
                self.query_changed();
                self.focus = Some(Field::Search);
            }
            Action::SetFilter(filter) => {
                self.search.filter = Some(filter);
                self.scroll.remove(&ScrollId::Main);
            }
            Action::Play(source, index) => self.play(&source, index),
            Action::PlayAll(source) => self.play_all(&source),
            Action::ShufflePlay(source) => {
                let tracks = self.tracks_of(&source);
                if tracks.is_empty() {
                    return;
                }
                if let Some(track) = self.queue.shuffle_play(&tracks) {
                    self.load(track);
                }
            }
            Action::Enqueue(track) => self.enqueue(track),
            Action::EnqueueFrom(source, index) => {
                if let Some(track) = self.track_at(&source, index) {
                    self.enqueue(track);
                }
            }
            Action::ToggleLike(track) => self.toggle_like(&track),
            Action::ToggleLikeFrom(source, index) => {
                if let Some(track) = self.track_at(&source, index) {
                    self.toggle_like(&track);
                }
            }
            Action::TrackMenu(source, index) => self.open_track_menu(&source, index),
            Action::CollectionMenu(id) => self.open_collection_menu(&id),
            Action::AddMenu => self.open_add_menu(),
            Action::DeviceMenu => self.open_device_menu(),
            Action::OpenSubmenu(index) => {
                if let Some(menu) = &mut self.menu {
                    menu.open_submenu = Some(index);
                }
            }
            Action::TogglePlay => self.toggle_play(),
            Action::Next => {
                if let Some(track) = self.queue.next(false) {
                    self.load(track);
                }
            }
            Action::Previous => self.previous(),
            Action::ToggleShuffle => self.queue.toggle_shuffle(),
            Action::CycleRepeat => self.queue.cycle_repeat(),
            Action::ToggleMute => {
                self.prefs.muted = !self.prefs.muted;
                self.apply_volume();
            }
            Action::ToggleQueue => {
                self.queue_open = !self.queue_open;
                self.repaint = true;
            }
            Action::ShuffleUpcoming => self.queue.shuffle_upcoming(),
            Action::ClearUpcoming => self.queue.clear_upcoming(),
            Action::CreatePlaylist => self.create_playlist(None),
            Action::OpenImport => self.open_dialog(Dialog::Import { link: String::new(), busy: false, error: None }),
            Action::StartRename(id) => {
                if let Some(collection) = self.collection(&id) {
                    let name = collection.name.clone();
                    self.open_dialog(Dialog::Rename { id, name });
                }
            }
            Action::AskDelete(id) => self.ask_delete(id),
            Action::Delete(id) => self.delete_collection(&id),
            Action::SetPinned(id, pinned) => {
                let result = self.services.library.set_pinned(&id, pinned);
                self.after_library_change(result);
            }
            Action::ResetCover(id) => {
                let result = self.services.library.set_cover(&id, None);
                self.after_library_change(result);
            }
            Action::AddToPlaylist(id, track) => self.add_to_playlist(&id, track),
            Action::AddToNewPlaylist(track) => self.create_playlist(Some(track)),
            Action::RemoveFromPlaylist(id, track) => {
                let result = self.services.library.remove_track(&id, &track).map(drop);
                self.after_library_change(result);
            }
            Action::OpenUrl(url) => {
                if let Err(err) = tauri_plugin_opener::open_url(url, None::<&str>) {
                    self.fail(err);
                }
            }
            Action::SetLocale(locale) => {
                self.prefs.locale = locale.to_owned();
                self.dict = Dict::load(locale);
                self.save_prefs();
            }
            Action::SetTheme(theme) => {
                self.prefs.theme = theme;
                self.restyle();
            }
            Action::SetAccent(accent) => {
                self.prefs.accent = accent.to_owned();
                self.restyle();
            }
            Action::ToggleAnimations => {
                self.prefs.animations = !self.prefs.animations;
                self.save_prefs();
            }
            Action::ToggleDiscord => {
                self.prefs.discord_presence = !self.prefs.discord_presence;
                self.services.presence.set_enabled(self.prefs.discord_presence);
                self.sync_presence();
                self.save_prefs();
            }
            Action::SetDevice(device) => {
                self.prefs.audio_device = device.clone();
                self.save_prefs();
                let engine = self.services.engine.clone();
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = engine.set_output_device(device).await {
                        let _ = tx.send(Message::Player(PlayerEvent::Error { message: err.to_string() }));
                    }
                });
            }
            Action::SwitchToWindow => self.exit = Some(Exit::Window),
            Action::DialogSubmit => self.submit_dialog(),
            Action::CloseOverlay => {
                if matches!(self.dialog, Some(Dialog::Import { busy: true, .. })) {
                    return;
                }
                if self.dialog.take().is_some() {
                    self.focus = None;
                    self.repaint = true;
                } else if self.focus.take().is_none() && self.can_back() {
                    self.dispatch(Action::Back);
                }
            }
            Action::Scroll(id, delta, max) => {
                let next = self.offset(id).saturating_add_signed(delta).min(max);
                self.scroll.insert(id, next);
            }
            Action::CloseMenu => {}
            Action::DismissToast(index) => {
                if index < self.toasts.len() {
                    self.toasts.remove(index);
                }
            }
        }
    }

    fn go(&mut self, route: Route) {
        if *self.route() != route {
            self.scroll.remove(&ScrollId::Main);
            self.history.go(route);
        }
    }

    fn restyle(&mut self) {
        self.theme = Theme::new(&self.prefs, self.rich_terminal);
        self.repaint = true;
        self.save_prefs();
    }

    pub fn save_prefs(&self) {
        // Best effort: a lost preference is not worth interrupting the music for.
        let _ = self.dirs.save_prefs(&UiPrefs { mode: Mode::Tui, ..self.prefs.clone() });
    }

    pub fn remember_mode(&self, mode: Mode) {
        let _ = self.dirs.save_prefs(&UiPrefs { mode, ..self.prefs.clone() });
    }

    fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
        self.focus = Some(Field::Dialog);
        self.repaint = true;
    }

    fn query_changed(&mut self) {
        self.go(Route::Search);
        if self.search.query.trim().is_empty() {
            self.search.request_id += 1;
            self.search.youtube = None;
            self.search.soundcloud = None;
            self.search.results_for.clear();
            self.search.pending = [false; 2];
            self.search.due = None;
        } else {
            self.search.due = Some(Instant::now() + SEARCH_DEBOUNCE);
        }
    }

    /// Providers answer independently; whichever is faster shows up first.
    fn run_search(&mut self) {
        self.search.due = None;
        let query = self.search.query.trim().to_owned();
        let busy = self.search.pending.iter().any(|p| *p);
        if query.is_empty() || (query == self.search.results_for && !busy) {
            return;
        }
        self.search.request_id += 1;
        let id = self.search.request_id;
        self.search.pending = [true; 2];
        self.scroll.remove(&ScrollId::Main);

        for provider in [Provider::Youtube, Provider::Soundcloud] {
            let services = self.services.clone();
            let tx = self.tx.clone();
            let query = query.clone();
            tauri::async_runtime::spawn(async move {
                let result = match provider {
                    Provider::Youtube => services.sources.search_youtube(&query).await,
                    _ => services.sources.search_soundcloud(&query).await,
                };
                let _ = tx.send(Message::Searched { id, provider, result });
            });
        }
    }

    pub fn on_message(&mut self, message: Message) {
        match message {
            Message::Player(event) => self.on_player_event(event),
            Message::Searched { id, provider, result } => {
                if id != self.search.request_id {
                    return;
                }
                let query = self.search.query.trim().to_owned();
                if self.search.results_for != query {
                    self.search.youtube = None;
                    self.search.soundcloud = None;
                    self.search.results_for = query;
                }
                self.search.pending[slot(provider)] = false;
                match provider {
                    Provider::Youtube => self.search.youtube = Some(result),
                    _ => self.search.soundcloud = Some(result),
                }
            }
            Message::Imported(result) => match result {
                Ok(collection) => {
                    self.dialog = None;
                    self.focus = None;
                    self.reload_library();
                    self.toast(format(&self.dict.importer.done, &[("name", &collection.name)]), false);
                    self.go(Route::Collection(collection.id));
                    self.repaint = true;
                }
                Err(err) => {
                    if let Some(Dialog::Import { busy, error, .. }) = &mut self.dialog {
                        *busy = false;
                        *error = Some(err.to_string());
                    }
                }
            },
            Message::Played => self.reload_library(),
            Message::PlayFailed { id, error } => {
                if id == self.play_id {
                    self.status = Status::Idle;
                    self.toast(format(&self.dict.errors.playback, &[("error", &error)]), true);
                }
            }
        }
    }

    fn on_player_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Playing { position_ms } => {
                self.engine_loaded = true;
                self.status = Status::Playing;
                self.position_ms = position_ms;
                self.sync_presence();
                self.prefetch_next();
            }
            PlayerEvent::Paused { position_ms } => {
                self.status = Status::Paused;
                self.position_ms = position_ms;
                self.sync_presence();
            }
            PlayerEvent::Progress { position_ms } => self.position_ms = position_ms,
            PlayerEvent::Ended => {
                self.engine_loaded = false;
                match self.queue.next(true) {
                    Some(track) => self.load(track),
                    None => {
                        self.status = Status::Idle;
                        self.position_ms = 0;
                        self.sync_presence();
                    }
                }
            }
            PlayerEvent::Error { message } => {
                self.toast(format(&self.dict.errors.playback, &[("error", &message)]), true);
            }
        }
    }

    fn play(&mut self, source: &Source, index: usize) {
        if *source == Source::Queue {
            if let Some(track) = self.queue.jump_to(index) {
                self.load(track);
            }
            return;
        }
        // Clicking the playing track pauses it, as the window's row button does.
        if let Some(track) = self.track_at(source, index) {
            if self.is_current(&track) && self.status != Status::Idle {
                self.toggle_play();
                return;
            }
        }
        let tracks = self.tracks_of(source);
        if let Some(track) = self.queue.play(&tracks, index) {
            self.load(track);
        }
    }

    pub fn is_playing_from(&self, source: &Source) -> bool {
        self.status == Status::Playing
            && self.queue.current().is_some_and(|current| {
                let key = key_of(current);
                self.tracks_of(source).iter().any(|t| key_of(t) == key)
            })
    }

    fn play_all(&mut self, source: &Source) {
        if self.is_playing_from(source) {
            self.toggle_play();
            return;
        }
        let tracks = self.tracks_of(source);
        if let Some(track) = self.queue.play(&tracks, 0) {
            self.load(track);
        }
    }

    fn enqueue(&mut self, track: Track) {
        if let Some(start) = self.queue.enqueue(track) {
            self.load(start);
        }
    }

    fn load(&mut self, track: Track) {
        self.status = Status::Loading;
        self.position_ms = 0;
        self.engine_loaded = false;
        self.play_id += 1;
        let id = self.play_id;
        let services = self.services.clone();
        let tx = self.tx.clone();

        tauri::async_runtime::spawn(async move {
            match crate::play_track(&services.sources, &services.engine, &track).await {
                Ok(()) => {
                    if services.library.mark_played(&track).is_ok() {
                        let _ = tx.send(Message::Played);
                    }
                }
                Err(Error::Cancelled) => {}
                Err(err) => {
                    let _ = tx.send(Message::PlayFailed { id, error: err.to_string() });
                }
            }
        });
    }

    /// Resolves the next stream ahead of time; the backend caches it, so skipping is instant.
    fn prefetch_next(&mut self) {
        let Some(next) = self.queue.upcoming().first().cloned() else { return };
        let key = key_of(&next);
        if self.prefetched.get(&key).is_some_and(|at| at.elapsed() < PREFETCH_TTL) {
            return;
        }
        self.prefetched.insert(key, Instant::now());
        let services = self.services.clone();
        tauri::async_runtime::spawn(async move {
            let _ = services.sources.stream(&next).await;
        });
    }

    fn toggle_play(&mut self) {
        match self.status {
            Status::Loading => {}
            Status::Playing => self.services.engine.pause(),
            _ if self.engine_loaded => self.services.engine.resume(),
            _ => {
                if let Some(track) = self.queue.current().cloned() {
                    self.load(track);
                }
            }
        }
    }

    fn previous(&mut self) {
        if self.position_ms > RESTART_THRESHOLD_MS {
            self.seek_to(0);
            return;
        }
        match self.queue.previous() {
            Some(track) => self.load(track),
            None => self.seek_to(0),
        }
    }

    fn seek_by(&mut self, delta_ms: i64) {
        self.seek_to(self.position_ms.saturating_add_signed(delta_ms));
    }

    fn seek_to(&mut self, position_ms: u64) {
        if self.queue.current().is_none() || !self.engine_loaded {
            return;
        }
        let duration = self.duration_ms();
        let target = if duration > 0 { position_ms.min(duration) } else { position_ms };
        self.services.engine.seek(Duration::from_millis(target));
        self.position_ms = target;
        if self.status == Status::Playing {
            self.sync_presence();
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.prefs.volume = volume.clamp(0.0, 1.0);
        self.prefs.muted = false;
        self.apply_volume();
    }

    fn apply_volume(&self) {
        let effective = if self.prefs.muted { 0.0 } else { self.prefs.volume };
        self.services.engine.set_volume(effective);
        self.save_prefs();
    }

    fn toggle_like(&mut self, track: &Track) {
        let liked = !self.is_liked(track);
        let result = self.services.library.set_liked(track, liked);
        self.after_library_change(result);
    }

    fn after_library_change(&mut self, result: Result<()>) {
        match result {
            Ok(()) => self.reload_library(),
            Err(err) => self.fail(err),
        }
    }

    fn reload_library(&mut self) {
        match self.services.library.snapshot() {
            Ok(snapshot) => {
                self.library = snapshot;
                self.index_liked();
            }
            Err(err) => self.fail(err),
        }
    }

    fn index_liked(&mut self) {
        self.liked = self.library.liked.iter().map(key_of).collect();
    }

    fn user_playlists(&self) -> impl Iterator<Item = &SavedCollection> {
        self.library.collections.iter().filter(|c| c.origin == Origin::User)
    }

    fn create_playlist(&mut self, first_track: Option<Track>) {
        let number = (self.user_playlists().count() + 1).to_string();
        let name = format(&self.dict.playlist.default_name, &[("number", &number)]);
        let created = match self.services.library.create_playlist(&name) {
            Ok(created) => created,
            Err(err) => return self.fail(err),
        };
        match first_track {
            Some(track) => self.add_to_playlist(&created.id, track),
            None => {
                self.reload_library();
                self.go(Route::Collection(created.id.clone()));
                self.dispatch(Action::StartRename(created.id));
            }
        }
    }

    fn add_to_playlist(&mut self, id: &str, track: Track) {
        match self.services.library.add_tracks(id, &[track]) {
            Ok(saved) => {
                self.reload_library();
                self.toast(format(&self.dict.playlist.added, &[("name", &saved.name)]), false);
            }
            Err(err) => self.fail(err),
        }
    }

    /// Imports can be brought back from their link; playlists made here cannot.
    fn ask_delete(&mut self, id: String) {
        let Some(collection) = self.collection(&id) else { return };
        if collection.origin != Origin::User {
            return self.delete_collection(&id);
        }
        let text = &self.dict.playlist;
        let dialog = Dialog::Confirm {
            title: format(&text.delete_title, &[("name", &collection.name)]),
            body: text.delete_body.clone(),
            confirm: text.delete.clone(),
            action: Box::new(Action::Delete(id)),
        };
        self.dialog = Some(dialog);
        self.repaint = true;
    }

    fn delete_collection(&mut self, id: &str) {
        self.dialog = None;
        match self.services.library.delete(id) {
            Ok(()) => {
                self.history.forget(&Route::Collection(id.to_owned()));
                self.reload_library();
                self.repaint = true;
            }
            Err(err) => self.fail(err),
        }
    }

    fn move_collection(&mut self, from: usize, to: usize) {
        let mut ids: Vec<String> = self.library.collections.iter().map(|c| c.id.clone()).collect();
        if from >= ids.len() || to >= ids.len() {
            return;
        }
        let moved = ids.remove(from);
        ids.insert(to, moved);
        let result = self.services.library.reorder_collections(&ids);
        self.after_library_change(result);
    }

    fn submit_dialog(&mut self) {
        match &mut self.dialog {
            Some(Dialog::Import { link, busy, error }) => {
                if *busy {
                    return;
                }
                let value = link.trim().to_owned();
                if detect_link(&value).is_none() {
                    *error = Some(self.dict.importer.invalid.clone());
                    return;
                }
                *busy = true;
                *error = None;
                let services = self.services.clone();
                let tx = self.tx.clone();
                tauri::async_runtime::spawn(async move {
                    let result = match services.sources.import(&value).await {
                        Ok(collection) => services.library.save_import(&collection, &value),
                        Err(err) => Err(err),
                    };
                    let _ = tx.send(Message::Imported(result));
                });
            }
            Some(Dialog::Rename { id, name }) => {
                let (id, name) = (id.clone(), name.trim().to_owned());
                self.dialog = None;
                self.focus = None;
                self.repaint = true;
                if !name.is_empty() {
                    let result = self.services.library.rename(&id, &name);
                    self.after_library_change(result);
                }
            }
            Some(Dialog::Confirm { action, .. }) => {
                let action = (**action).clone();
                self.dispatch(action);
            }
            None => {}
        }
    }

    fn open_menu(&mut self, items: Vec<MenuItem>) {
        let anchor = self.mouse.unwrap_or_default();
        self.menu = Some(Menu { anchor, items, open_submenu: None });
    }

    fn open_track_menu(&mut self, source: &Source, index: usize) {
        let Some(track) = self.track_at(source, index) else { return };
        let icons = self.theme.icons;
        let text = &self.dict;
        let playlist_id = match source {
            Source::Collection(id) if self.collection(id).is_some_and(|c| c.origin == Origin::User) => Some(id.clone()),
            _ => None,
        };

        let mut playlists = vec![MenuItem::new(icons.add, &text.playlist.new_playlist, Action::AddToNewPlaylist(track.clone()))];
        playlists.extend(
            self.user_playlists()
                .filter(|p| Some(&p.id) != playlist_id.as_ref())
                .map(|p| MenuItem::new(icons.note, &p.name, Action::AddToPlaylist(p.id.clone(), track.clone()))),
        );
        let liked = self.is_liked(&track);
        let mut items = vec![
            MenuItem::new(icons.queue, &text.track.add_to_queue, Action::Enqueue(track.clone())),
            MenuItem {
                icon: icons.add,
                label: text.playlist.add_to.clone(),
                danger: false,
                checked: false,
                action: None,
                submenu: playlists,
            },
            MenuItem::new(
                icons.liked,
                if liked { &text.track.unlike } else { &text.track.like },
                Action::ToggleLike(track.clone()),
            ),
        ];
        if let Some(id) = playlist_id {
            let mut remove = MenuItem::new(icons.trash, &text.playlist.remove_from, Action::RemoveFromPlaylist(id, track.clone()));
            remove.danger = true;
            items.push(remove);
        }
        if let Some(url) = &track.url {
            let label = format(&text.track.open_source, &[("provider", text.provider(track.provider))]);
            items.push(MenuItem::new(icons.link, label, Action::OpenUrl(url.clone())));
        }
        self.open_menu(items);
    }

    fn open_collection_menu(&mut self, id: &str) {
        let Some(collection) = self.collection(id) else { return };
        let icons = self.theme.icons;
        let text = &self.dict;
        let id = collection.id.clone();
        let mut items = vec![if collection.pinned {
            MenuItem::new(icons.pin, &text.playlist.unpin, Action::SetPinned(id.clone(), false))
        } else {
            MenuItem::new(icons.pin, &text.playlist.pin, Action::SetPinned(id.clone(), true))
        }];
        if collection.origin == Origin::User {
            items.push(MenuItem::new(icons.rename, &text.playlist.rename, Action::StartRename(id.clone())));
        }
        if collection.custom_artwork.is_some() {
            items.push(MenuItem::new(icons.album, &text.playlist.reset_cover, Action::ResetCover(id.clone())));
        }
        let label = if collection.origin == Origin::User { &text.playlist.delete } else { &text.collection.remove };
        let mut delete = MenuItem::new(icons.trash, label, Action::AskDelete(id));
        delete.danger = true;
        items.push(delete);
        self.open_menu(items);
    }

    fn open_add_menu(&mut self) {
        let icons = self.theme.icons;
        let items = vec![
            MenuItem::new(icons.note, &self.dict.playlist.create, Action::CreatePlaylist),
            MenuItem::new(icons.import, &self.dict.importer.action, Action::OpenImport),
        ];
        self.open_menu(items);
    }

    fn open_device_menu(&mut self) {
        self.devices = devices::list();
        let chosen = self.prefs.audio_device.clone();
        let chosen_present = self.devices.iter().any(|d| Some(&d.id) == chosen.as_ref());
        let default_label = match self.devices.iter().find(|d| d.is_default) {
            Some(device) => format!("{} ({})", self.dict.settings.output_default, device.name),
            None => self.dict.settings.output_default.clone(),
        };
        let mut items = vec![MenuItem {
            checked: !chosen_present,
            ..MenuItem::new(self.theme.icons.volume_high, default_label, Action::SetDevice(None))
        }];
        items.extend(self.devices.iter().map(|device| MenuItem {
            checked: chosen.as_deref() == Some(device.id.as_str()),
            ..MenuItem::new("", &device.name, Action::SetDevice(Some(device.id.clone())))
        }));
        self.open_menu(items);
    }

    pub fn output_label(&self) -> String {
        let chosen = self.prefs.audio_device.as_deref();
        let devices = if self.devices.is_empty() { devices::list() } else { self.devices.clone() };
        match chosen.and_then(|id| devices.iter().find(|d| d.id == id)) {
            Some(device) => device.name.clone(),
            None => match devices.iter().find(|d| d.is_default) {
                Some(device) => format!("{} ({})", self.dict.settings.output_default, device.name),
                None => self.dict.settings.output_default.clone(),
            },
        }
    }

    /// Discord shows only what is actually playing, like the window does.
    fn sync_presence(&self) {
        let presence = &self.services.presence;
        match (self.status, self.queue.current()) {
            (Status::Playing, Some(track)) => presence.show(NowPlaying {
                title: track.title.clone(),
                artists: track.artists.clone(),
                album: track.album.clone(),
                artwork: track.artwork.clone(),
                url: track.url.clone(),
                position_ms: self.position_ms,
                duration_ms: track.duration_ms,
            }),
            _ => presence.hide(),
        }
    }

    pub fn shutdown(&self) {
        self.services.presence.hide();
        self.services.engine.stop();
    }
}

/// Mirrors the window's link checks so obvious mistakes are caught before a request.
pub fn detect_link(link: &str) -> Option<Provider> {
    let lower = link.to_ascii_lowercase();
    let without_scheme = lower.trim_start_matches("https://").trim_start_matches("http://");
    let host = without_scheme.split('/').next().unwrap_or_default();
    let path = &without_scheme[host.len()..];

    if lower.starts_with("spotify:playlist:") || lower.starts_with("spotify:album:") {
        return Some(Provider::Spotify);
    }
    match host {
        "open.spotify.com" if path.contains("/playlist/") || path.contains("/album/") => Some(Provider::Spotify),
        "soundcloud.com" | "www.soundcloud.com" | "m.soundcloud.com" if path.contains("/sets/") => {
            Some(Provider::Soundcloud)
        }
        _ if (host.ends_with("youtube.com") || host == "youtu.be")
            && (path.contains("list=") || path.contains("/browse/mpreb")) =>
        {
            Some(Provider::Youtube)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_playlist_links() {
        assert_eq!(detect_link("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"), Some(Provider::Spotify));
        assert_eq!(detect_link("spotify:album:abc"), Some(Provider::Spotify));
        assert_eq!(detect_link("https://soundcloud.com/user/sets/mix"), Some(Provider::Soundcloud));
        assert_eq!(detect_link("https://music.youtube.com/playlist?list=PL123"), Some(Provider::Youtube));
        assert_eq!(detect_link("https://www.youtube.com/watch?v=abc"), None);
        assert_eq!(detect_link("hello"), None);
    }

    #[test]
    fn history_behaves_like_a_browser() {
        let mut history = History { stack: vec![Route::Home], index: 0 };
        history.go(Route::Search);
        history.go(Route::Collection("a".into()));
        history.index = 1;
        history.go(Route::Liked);
        assert_eq!(history.stack, [Route::Home, Route::Search, Route::Liked]);

        history.go(Route::Collection("a".into()));
        history.forget(&Route::Collection("a".into()));
        assert_eq!(history.current(), &Route::Liked);
    }
}
