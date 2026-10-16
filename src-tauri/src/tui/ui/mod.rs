//! Drawing. Every frame lays out the window's structure and registers what each region
//! does when clicked, so the terminal is driven with the mouse like the window is.
//!
//! Text stays text; icons, rounded corners, round buttons and sliders are small pictures
//! fitted to whole cells, each painted on the background it sits on.

mod chrome;
mod overlays;
mod pages;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use super::app::{Action, App, Hit, Hits, Slider};
use super::art::Art;
use super::paint::{self, Corner, Icon, Visual, BAR_LEVELS, SLIDER_STEPS};
use super::theme::{provider_color, Theme};
use crate::model::Provider;

const TITLEBAR_HEIGHT: u16 = 3;
const PLAYERBAR_HEIGHT: u16 = 5;
const SIDEBAR_WIDTH: u16 = 34;
const QUEUE_WIDTH: u16 = 40;
const GAP: u16 = 1;
/// An icon covers two cells: 20×20 pixels in Windows Terminal's 10×20 cell.
pub const ICON_WIDTH: u16 = 2;
/// A small icon button: the icon with a cell of padding on each side.
pub const BUTTON_WIDTH: u16 = 4;
const SWITCH_WIDTH: u16 = 4;
/// Hovered accent buttons lighten by this much, as the window's do.
const HOVER_LIGHTEN: f32 = 0.12;
const WHITE: paint::Rgb = [255, 255, 255];
/// Start of a Sixel image (DCS) inside a cell's symbol.
const SIXEL_START: &str = "\x1bP";
/// Save the cursor before a picture, and after it return to where it started, one cell on.
/// Windows Terminal moves the cursor below a Sixel image, while ratatui assumes it sits
/// right after the cell; without this, text drawn next lands in the wrong place.
const CURSOR_SAVE: &str = "\x1b7";
const CURSOR_AFTER_PICTURE: &str = "\x1b8\x1b[1C";

pub struct Ui<'a, 'f> {
    frame: &'a mut Frame<'f>,
    app: &'a App,
    art: &'a mut Art,
    hits: &'a mut Hits,
    /// Advances while music plays, driving the "now playing" bars.
    beat: u64,
}

/// What stands in for a cover that has not loaded or does not exist.
#[derive(Clone, Copy)]
pub enum Fallback {
    Notes,
    Liked,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PillStyle {
    Primary,
    Secondary,
    /// A chosen filter chip: inverted, like the window's.
    Selected,
}

pub fn render(frame: &mut Frame, app: &App, art: &mut Art, hits: &mut Hits, beat: u64) {
    hits.clear();
    let mut ui = Ui { frame, app, art, hits, beat };
    let area = ui.frame.area();
    ui.fill(area, app.theme.bg);

    let [titlebar, _, middle, playerbar] = Layout::vertical([
        Constraint::Length(TITLEBAR_HEIGHT),
        Constraint::Length(GAP),
        Constraint::Min(8),
        Constraint::Length(PLAYERBAR_HEIGHT),
    ])
    .areas(area);

    let queue_width = if app.queue_open { QUEUE_WIDTH + GAP } else { 0 };
    let [_, sidebar, _, main, queue, _] = Layout::horizontal([
        Constraint::Length(GAP),
        Constraint::Length(SIDEBAR_WIDTH),
        Constraint::Length(GAP),
        Constraint::Min(30),
        Constraint::Length(queue_width),
        Constraint::Length(GAP),
    ])
    .areas(middle);

    chrome::titlebar(&mut ui, titlebar);
    chrome::sidebar(&mut ui, sidebar);
    pages::main(&mut ui, main);
    if app.queue_open {
        let [_, panel] = Layout::horizontal([Constraint::Length(GAP), Constraint::Min(0)]).areas(queue);
        chrome::queue(&mut ui, panel);
    }
    chrome::playerbar(&mut ui, playerbar);
    overlays::toasts(&mut ui, middle);
    overlays::menu(&mut ui, area);
    overlays::dialog(&mut ui, area);
}

fn lighten(color: Color) -> Color {
    let [r, g, b] = paint::mix(paint::rgb(color), WHITE, HOVER_LIGHTEN);
    Color::Rgb(r, g, b)
}

impl<'a> Ui<'a, '_> {
    /// Borrowed from the app rather than from `self`, so it stays usable while drawing.
    fn theme(&self) -> &'a Theme {
        &self.app.theme
    }

    fn fill(&mut self, area: Rect, bg: Color) {
        self.frame.render_widget(Block::new().style(Style::new().bg(bg)), area);
    }

    fn line(&mut self, area: Rect, line: Line) {
        self.frame.render_widget(line, area);
    }

    /// One line of text at `x, y`, cut with an ellipsis to `width`; returns the width used.
    fn text(&mut self, x: u16, y: u16, width: u16, text: &str, style: Style) -> u16 {
        let shown = ellipsize(text, width as usize);
        let used = (display_width(&shown) as u16).min(width);
        self.line(Rect::new(x, y, used, 1), Line::styled(shown, style));
        used
    }

    /// Draws a picture; `false` on consoles that cannot, so the caller falls back to text.
    fn picture(&mut self, area: Rect, visual: Visual) -> bool {
        match self.art.visual(visual, area) {
            Some(protocol) => {
                self.frame.render_widget(ratatui_image::Image::new(protocol), area);
                self.anchor_cursor(area);
                true
            }
            None => false,
        }
    }

    fn anchor_cursor(&mut self, area: Rect) {
        let Some(cell) = self.frame.buffer_mut().cell_mut((area.x, area.y)) else { return };
        if cell.symbol().contains(SIXEL_START) {
            let wrapped = format!("{CURSOR_SAVE}{}{CURSOR_AFTER_PICTURE}", cell.symbol());
            cell.set_symbol(&wrapped);
        }
    }

    /// An icon in two cells at `x, y`.
    fn glyph(&mut self, x: u16, y: u16, icon: Icon, fg: Color, bg: Color) {
        let area = Rect::new(x, y, ICON_WIDTH, 1);
        let visual = Visual::Icon { icon, fg: paint::rgb(fg), bg: paint::rgb(bg) };
        if !self.picture(area, visual) {
            self.line(area, Line::styled(format!("{} ", icon.glyph()), Style::new().fg(fg).bg(bg)));
        }
    }

    /// Paints `area` in `fill` with corners rounded onto `outside`, like the window's panels.
    fn rounded(&mut self, area: Rect, fill: Color, outside: Color) {
        if area.is_empty() {
            return;
        }
        self.fill(area, fill);
        let (fill, bg) = (paint::rgb(fill), paint::rgb(outside));
        let corners = if area.height == 1 {
            vec![(Corner::Left, area.x, area.y), (Corner::Right, area.right() - 1, area.y)]
        } else {
            vec![
                (Corner::TopLeft, area.x, area.y),
                (Corner::TopRight, area.right() - 1, area.y),
                (Corner::BottomLeft, area.x, area.bottom() - 1),
                (Corner::BottomRight, area.right() - 1, area.bottom() - 1),
            ]
        };
        for (corner, x, y) in corners {
            self.picture(Rect::new(x, y, 1, 1), Visual::Corner { corner, fill, bg });
        }
    }

    /// An icon button with a rounded highlight under the mouse; returns its width.
    fn icon_button(&mut self, x: u16, y: u16, icon: Icon, color: Color, bg: Color, action: Action) -> u16 {
        let area = Rect::new(x, y, BUTTON_WIDTH, 1);
        let t = self.theme();
        let hovered = self.app.hovered(area);
        let under = if hovered { t.hover } else { bg };
        if hovered {
            self.rounded(area, t.hover, bg);
        } else {
            self.fill(area, bg);
        }
        let color = if hovered && color == t.muted { t.text } else { color };
        self.glyph(x + 1, y, icon, color, under);
        self.hits.click(area, action);
        BUTTON_WIDTH
    }

    /// A circular button like the window's play buttons; `area` should be square in pixels.
    /// A `fill` equal to `bg` gives a plain icon that only shows its circle on hover.
    fn round_button(&mut self, area: Rect, icon: Icon, fill: Color, fg: Color, bg: Color, action: Action) {
        let t = self.theme();
        let hovered = self.app.hovered(area);
        let fill = match (hovered, fill == bg) {
            (false, _) => fill,
            (true, true) => t.hover,
            (true, false) => lighten(fill),
        };
        let fg = if hovered && fg == t.muted { t.text } else { fg };
        let visual = Visual::Round { icon, fill: paint::rgb(fill), fg: paint::rgb(fg), bg: paint::rgb(bg) };
        if !self.picture(area, visual) {
            self.fill(area, fill);
            let middle = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
            self.line(middle, Line::styled(icon.glyph(), Style::new().fg(fg).bg(fill)).centered());
        }
        self.hits.click(area, action);
    }

    /// A rounded pill button with an optional icon; returns its width.
    #[allow(clippy::too_many_arguments)]
    fn pill(&mut self, x: u16, y: u16, icon: Option<Icon>, label: &str, style: PillStyle, bg: Color, action: Action) -> u16 {
        let width = pill_width(icon, label);
        let area = Rect::new(x, y, width, 1);
        let t = self.theme();
        let hovered = self.app.hovered(area);
        let (fill, fg) = match (style, hovered) {
            (PillStyle::Primary, false) => (t.accent, t.on_accent),
            (PillStyle::Primary, true) => (lighten(t.accent), t.on_accent),
            (PillStyle::Selected, _) => (t.text, t.bg),
            (PillStyle::Secondary, false) => (t.active, t.text),
            (PillStyle::Secondary, true) => (t.hover, t.text),
        };
        self.rounded(area, fill, bg);
        let mut cursor = x + 2;
        if let Some(icon) = icon {
            self.glyph(cursor, y, icon, fg, fill);
            cursor += ICON_WIDTH + 1;
        }
        let weight = if style == PillStyle::Secondary { Modifier::empty() } else { Modifier::BOLD };
        let room = area.right().saturating_sub(cursor + 2);
        self.text(cursor, y, room, label, Style::new().fg(fg).bg(fill).add_modifier(weight));
        self.hits.click(area, action);
        width
    }

    /// A cover with rounded corners, or the window's placeholder while it loads or is missing.
    fn cover(&mut self, area: Rect, url: Option<&str>, fallback: Fallback, bg: Color) {
        if let Some(protocol) = url.and_then(|url| self.art.cover(url, area, paint::rgb(bg))) {
            self.frame.render_widget(ratatui_image::Image::new(protocol), area);
            self.anchor_cursor(area);
            return;
        }
        let t = self.theme();
        let visual = match fallback {
            Fallback::Notes => Visual::Notes { fill: paint::rgb(t.active), fg: paint::rgb(t.faint), bg: paint::rgb(bg) },
            Fallback::Liked => {
                Visual::Liked { accent: paint::rgb(t.accent), surface: paint::rgb(t.surface), bg: paint::rgb(bg) }
            }
        };
        if self.picture(area, visual) {
            return;
        }
        let (fill, icon, fg) = match fallback {
            Fallback::Notes => (t.active, Icon::Notes, t.faint),
            Fallback::Liked => (t.accent, Icon::HeartFill, t.on_accent),
        };
        self.fill(area, fill);
        let middle = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
        self.line(middle, Line::styled(icon.glyph(), Style::new().fg(fg).bg(fill)).centered());
    }

    /// The service's logo and name, as the window's provider badge; returns the width used.
    fn badge(&mut self, x: u16, y: u16, width: u16, provider: Provider, bg: Color) -> u16 {
        if width < ICON_WIDTH {
            return 0;
        }
        self.glyph(x, y, Icon::provider(provider), provider_color(provider), bg);
        let name = self.app.dict.provider(provider).to_owned();
        let style = Style::new().fg(self.theme().faint).bg(bg);
        ICON_WIDTH + 1 + self.text(x + ICON_WIDTH + 1, y, width.saturating_sub(ICON_WIDTH + 1), &name, style)
    }

    fn slider(&mut self, area: Rect, ratio: f64, kind: Slider, enabled: bool, bg: Color) {
        if area.width < 2 {
            return;
        }
        let t = self.theme();
        let hovered = self.app.hovered(area);
        let dragging = kind == Slider::Seek && self.app.scrub_ms.is_some();
        let active = hovered || dragging;
        let played = match (enabled, active) {
            (false, _) => t.faint,
            (true, true) => t.accent,
            (true, false) => t.text,
        };
        let ratio = ratio.clamp(0.0, 1.0);
        let visual = Visual::Slider {
            filled: (ratio * SLIDER_STEPS as f64).round() as u16,
            knob: enabled && active,
            played: paint::rgb(played),
            rest: paint::rgb(t.active),
            bg: paint::rgb(bg),
        };
        if !self.picture(area, visual) {
            let done = ((area.width - 1) as f64 * ratio).round() as usize;
            let rest = (area.width as usize).saturating_sub(done + 1);
            self.line(
                area,
                Line::from(vec![
                    Span::styled("━".repeat(done + 1), Style::new().fg(played).bg(bg)),
                    Span::styled("━".repeat(rest), Style::new().fg(t.active).bg(bg)),
                ]),
            );
        }
        if enabled {
            self.hits.add(Hit { area, slider: Some(kind), ..Hit::default() });
        }
    }

    fn switch(&mut self, x: u16, y: u16, on: bool, bg: Color, action: Action) {
        let t = self.theme();
        let area = Rect::new(x, y, SWITCH_WIDTH, 1);
        let (track, knob) = if on { (t.accent, t.on_accent) } else { (t.active, t.text) };
        let visual = Visual::Switch { on, track: paint::rgb(track), knob: paint::rgb(knob), bg: paint::rgb(bg) };
        if !self.picture(area, visual) {
            let text = if on { "  ██" } else { "██  " };
            self.line(area, Line::styled(text, Style::new().fg(knob).bg(track)));
        }
        self.hits.click(area, action);
    }

    /// Options in a rounded track with the chosen one raised, like the window's segmented control.
    fn segmented<T: Copy + PartialEq>(
        &mut self,
        x: u16,
        y: u16,
        options: &[(T, String)],
        chosen: T,
        bg: Color,
        action: impl Fn(T) -> Action,
    ) {
        let t = self.theme();
        let total: u16 = options.iter().map(|(_, label)| segment_width(label)).sum();
        self.rounded(Rect::new(x, y, total, 1), t.hover, bg);
        let mut cursor = x;
        for (value, label) in options {
            let width = segment_width(label);
            let area = Rect::new(cursor, y, width, 1);
            let style = if *value == chosen {
                self.rounded(area, t.active, t.hover);
                Style::new().fg(t.text).bg(t.active).add_modifier(Modifier::BOLD)
            } else if self.app.hovered(area) {
                Style::new().fg(t.text).bg(t.hover)
            } else {
                Style::new().fg(t.muted).bg(t.hover)
            };
            self.text(cursor + 2, y, width - 4, label, style);
            self.hits.click(area, action(*value));
            cursor += width;
        }
    }

    fn heading(&mut self, x: u16, y: u16, width: u16, text: &str, bg: Color) {
        let style = Style::new().fg(self.theme().text).bg(bg).add_modifier(Modifier::BOLD);
        self.text(x, y, width, text, style);
    }

    /// The window's "now playing" bars; they move while music plays and animations are on.
    fn bars(&mut self, x: u16, y: u16, playing: bool, bg: Color) {
        let t = self.theme();
        let levels = if playing && self.app.prefs().animations {
            let beat = self.beat;
            [0u64, 1, 2].map(|i| ((beat * (i + 2) + i * 3) % BAR_LEVELS as u64) as u8)
        } else {
            [3, 6, 2]
        };
        let visual = Visual::Bars { levels, color: paint::rgb(t.accent), bg: paint::rgb(bg) };
        if !self.picture(Rect::new(x, y, ICON_WIDTH, 1), visual) {
            self.text(x, y, ICON_WIDTH, "♪", Style::new().fg(t.accent).bg(bg));
        }
    }
}

pub fn pill_width(icon: Option<Icon>, label: &str) -> u16 {
    let icon_width = if icon.is_some() { ICON_WIDTH + 1 } else { 0 };
    display_width(label) as u16 + icon_width + 4
}

pub fn segment_width(label: &str) -> u16 {
    display_width(label) as u16 + 4
}

pub fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

/// Cuts text to `width` columns, ending with an ellipsis when something was cut.
pub fn ellipsize(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut used = 0;
    let mut out = String::new();
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out.push('…');
    out
}

/// The end of `text` that fits in `width` columns, so typing stays visible in short fields.
pub fn tail(text: &str, width: usize) -> String {
    let mut kept = Vec::new();
    let mut used = 0;
    for c in text.chars().rev() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        used += w;
        kept.push(c);
    }
    kept.into_iter().rev().collect()
}

pub fn clock(ms: Option<u64>) -> String {
    let Some(ms) = ms else { return "-:--".into() };
    let seconds = ms / 1000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations_like_the_window() {
        assert_eq!(clock(None), "-:--");
        assert_eq!(clock(Some(369_000)), "6:09");
        assert_eq!(clock(Some(3_725_000)), "1:02:05");
    }

    #[test]
    fn ellipsizes_by_display_width() {
        assert_eq!(ellipsize("Get Lucky", 20), "Get Lucky");
        assert_eq!(ellipsize("Get Lucky", 5), "Get …");
        assert_eq!(ellipsize("日本語テキスト", 5), "日本…");
    }
}
