//! Drawing. Every frame lays out the window's structure and registers what each region
//! does when clicked, so the terminal is driven with the mouse like the window is.

mod chrome;
mod overlays;
mod pages;

use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use super::app::{Action, App, Hit, Hits, Slider};
use super::art::Art;
use super::theme::{provider_color, Theme};
use crate::model::Provider;

const TITLEBAR_HEIGHT: u16 = 3;
const PLAYERBAR_HEIGHT: u16 = 5;
const SIDEBAR_WIDTH: u16 = 32;
const QUEUE_WIDTH: u16 = 38;
const GAP: u16 = 1;
const SWITCH_WIDTH: u16 = 6;

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
    Note,
    Liked,
}

pub fn render(frame: &mut Frame, app: &App, art: &mut Art, hits: &mut Hits, beat: u64) {
    hits.clear();
    let mut ui = Ui { frame, app, art, hits, beat };
    let area = ui.frame.area();
    ui.fill(area, app.theme.bg);

    let [titlebar, middle, playerbar] = Layout::vertical([
        Constraint::Length(TITLEBAR_HEIGHT),
        Constraint::Min(6),
        Constraint::Length(PLAYERBAR_HEIGHT),
    ])
    .areas(area);

    let queue_width = if app.queue_open { QUEUE_WIDTH + GAP } else { 0 };
    let [sidebar, _, main, queue] = Layout::horizontal([
        Constraint::Length(SIDEBAR_WIDTH),
        Constraint::Length(GAP),
        Constraint::Min(20),
        Constraint::Length(queue_width),
    ])
    .areas(middle.inner(Margin::new(GAP, 0)));

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

    /// A clickable region that lights up under the mouse.
    fn button(&mut self, area: Rect, content: Line, normal: Style, hover: Style, action: Action) {
        let style = if self.app.hovered(area) { hover } else { normal };
        self.frame.render_widget(Block::new().style(style), area);
        self.line(area, content.style(style));
        self.hits.click(area, action);
    }

    /// A small symbol button; `on` paints it in the accent, like toggled icons in the window.
    fn icon(&mut self, x: u16, y: u16, glyph: &str, on: bool, action: Action) -> u16 {
        let width = display_width(glyph) as u16 + 2;
        let area = Rect::new(x, y, width, 1);
        let t = self.theme();
        let normal = Style::new().fg(if on { t.accent } else { t.muted });
        let hover = Style::new().fg(if on { t.accent } else { t.text }).bg(t.hover);
        self.button(area, Line::from(format!(" {glyph} ")), normal, hover, action);
        width
    }

    /// A pill button: the label padded with spaces on a solid background.
    fn pill(&mut self, x: u16, y: u16, label: &str, primary: bool, action: Action) -> u16 {
        let width = display_width(label) as u16 + 4;
        let area = Rect::new(x, y, width, 1);
        let t = self.theme();
        let bold = Modifier::BOLD;
        let (normal, hover) = if primary {
            (Style::new().fg(t.on_accent).bg(t.accent).add_modifier(bold), Style::new().fg(t.bg).bg(t.text).add_modifier(bold))
        } else {
            (Style::new().fg(t.text).bg(t.active), Style::new().fg(t.text).bg(t.hover).add_modifier(bold))
        };
        self.button(area, Line::from(format!("  {label}  ")), normal, hover, action);
        width
    }

    fn art(&mut self, area: Rect, url: Option<&str>, fallback: Fallback) {
        if let Some(protocol) = url.and_then(|url| self.art.get(url, area)) {
            self.frame.render_widget(ratatui_image::Image::new(protocol), area);
            return;
        }
        let t = self.theme();
        let (bg, glyph, fg) = match fallback {
            Fallback::Note => (t.active, t.icons.note, t.faint),
            Fallback::Liked => (t.accent, t.icons.liked, t.on_accent),
        };
        self.fill(area, bg);
        let middle = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
        self.line(middle, Line::styled(glyph, Style::new().fg(fg).bg(bg)).centered());
    }

    fn badge(&mut self, x: u16, y: u16, width: u16, provider: Provider) -> u16 {
        let name = self.app.dict.provider(provider).to_owned();
        let line = Line::from(vec![
            Span::styled("● ", Style::new().fg(provider_color(provider))),
            Span::styled(name, Style::new().fg(self.theme().faint)),
        ]);
        let used = (line.width() as u16).min(width);
        self.line(Rect::new(x, y, used, 1), line);
        used
    }

    fn slider(&mut self, area: Rect, ratio: f64, kind: Slider, enabled: bool) {
        if area.width < 2 {
            return;
        }
        let t = self.theme();
        let hovered = self.app.hovered(area);
        let filled = ((area.width - 1) as f64 * ratio.clamp(0.0, 1.0)).round() as usize;
        let rest = (area.width as usize).saturating_sub(filled + 1);
        let played = match (enabled, hovered) {
            (false, _) => t.faint,
            (true, true) => t.accent,
            (true, false) => t.text,
        };
        let dragging = kind == Slider::Seek && self.app.scrub_ms.is_some();
        let knob = if enabled && (hovered || dragging) { "●" } else { "━" };
        self.line(
            area,
            Line::from(vec![
                Span::styled("━".repeat(filled), Style::new().fg(played)),
                Span::styled(knob, Style::new().fg(if enabled { played } else { t.active })),
                Span::styled("━".repeat(rest), Style::new().fg(t.active)),
            ]),
        );
        if enabled {
            self.hits.add(Hit { area, slider: Some(kind), ..Hit::default() });
        }
    }

    /// An on/off switch: the knob slides right and the track turns accent when on.
    fn switch(&mut self, x: u16, y: u16, on: bool, action: Action) {
        let t = self.theme();
        let area = Rect::new(x, y, SWITCH_WIDTH, 1);
        let (track, knob) = if on { (t.accent, t.on_accent) } else { (t.active, t.text) };
        let knob = Span::styled("██", Style::new().fg(knob).bg(track));
        let gap = Span::styled("    ", Style::new().bg(track));
        let line = if on { Line::from(vec![gap, knob]) } else { Line::from(vec![knob, gap]) };
        self.line(area, line);
        self.hits.click(area, action);
    }

    /// A row of options with the chosen one raised, like the window's segmented controls.
    fn segmented<T: Copy + PartialEq>(
        &mut self,
        x: u16,
        y: u16,
        options: &[(T, String)],
        chosen: T,
        action: impl Fn(T) -> Action,
    ) {
        let t = self.theme();
        let base = Style::new().fg(t.muted).bg(t.hover);
        let selected = Style::new().fg(t.text).bg(t.active).add_modifier(Modifier::BOLD);
        let hover = Style::new().fg(t.text).bg(t.hover);
        let mut cursor = x;
        for (value, label) in options {
            let width = display_width(label) as u16 + 4;
            let area = Rect::new(cursor, y, width, 1);
            let (normal, over) = if *value == chosen { (selected, selected) } else { (base, hover) };
            self.button(area, Line::from(format!("  {label}  ")), normal, over, action(*value));
            cursor += width;
        }
    }

    fn heading(&mut self, x: u16, y: u16, width: u16, text: &str) {
        let style = Style::new().fg(self.theme().text).add_modifier(Modifier::BOLD);
        self.text(x, y, width, text, style);
    }

    /// Animated "now playing" bars, frozen while paused or with animations off.
    fn bars(&self, playing: bool) -> String {
        const LEVELS: [char; 6] = ['▁', '▂', '▃', '▅', '▆', '▇'];
        if !playing || !self.app.prefs().animations {
            return "▃▅▂".into();
        }
        (0..3u64)
            .map(|i| LEVELS[((self.beat * (i + 2) + i * 3) % LEVELS.len() as u64) as usize])
            .collect()
    }
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
