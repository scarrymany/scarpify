//! The window's dictionaries (src/lib/i18n/*.json), read by the terminal mode as well, so
//! both interfaces always say the same thing.

use serde::Deserialize;

const EN: &str = include_str!("../../../src/lib/i18n/en.json");
const RU: &str = include_str!("../../../src/lib/i18n/ru.json");
const UK: &str = include_str!("../../../src/lib/i18n/uk.json");

pub const LOCALES: [(&str, &str); 3] = [("uk", "Українська"), ("ru", "Русский"), ("en", "English")];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dict {
    pub nav: Nav,
    pub home: Home,
    pub search: Search,
    pub track: TrackText,
    pub collection: CollectionText,
    pub playlist: Playlist,
    pub importer: Importer,
    pub player: Player,
    pub queue: QueueText,
    pub settings: Settings,
    pub errors: Errors,
    pub providers: Providers,
    pub plural: Plural,
    pub tui: Tui,
    #[serde(skip)]
    locale: &'static str,
}

#[derive(Deserialize)]
pub struct Nav {
    pub home: String,
    pub search: String,
    pub library: String,
    pub liked: String,
    pub settings: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub morning: String,
    pub afternoon: String,
    pub evening: String,
    pub night: String,
    pub recent: String,
    pub playlists: String,
    pub empty_title: String,
    pub empty_body: String,
    pub search_action: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    pub placeholder: String,
    pub all: String,
    pub tracks: String,
    pub top_result: String,
    pub no_results: String,
    pub no_results_hint: String,
    pub idle_title: String,
    pub idle_body: String,
    pub provider_error: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackText {
    pub title: String,
    pub album: String,
    pub like: String,
    pub unlike: String,
    pub add_to_queue: String,
    pub open_source: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionText {
    pub playlist: String,
    pub album: String,
    pub imported_from: String,
    pub remove: String,
    pub empty: String,
    pub liked_empty_title: String,
    pub liked_empty_body: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub default_name: String,
    pub create: String,
    pub added: String,
    pub add_to: String,
    pub new_playlist: String,
    pub remove_from: String,
    pub rename: String,
    pub reset_cover: String,
    pub delete: String,
    pub delete_title: String,
    pub delete_body: String,
    pub pin: String,
    pub unpin: String,
    pub empty_title: String,
    pub empty_body: String,
}

#[derive(Deserialize)]
pub struct Importer {
    pub action: String,
    pub title: String,
    pub body: String,
    pub placeholder: String,
    pub submit: String,
    pub cancel: String,
    pub working: String,
    pub invalid: String,
    pub done: String,
}

#[derive(Deserialize)]
pub struct Player {
    pub nothing: String,
    pub loading: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueText {
    pub title: String,
    pub now_playing: String,
    pub next: String,
    pub clear: String,
    pub empty: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub title: String,
    pub language: String,
    pub appearance: String,
    pub theme: String,
    pub theme_dark: String,
    pub theme_light: String,
    pub theme_system: String,
    pub accent: String,
    pub motion: String,
    pub motion_hint: String,
    pub discord: String,
    pub discord_hint: String,
    pub output: String,
    pub output_default: String,
    pub output_hint: String,
    pub about: String,
    pub about_body: String,
    pub source_code: String,
}

#[derive(Deserialize)]
pub struct Errors {
    pub playback: String,
    pub generic: String,
}

#[derive(Deserialize)]
pub struct Providers {
    pub youtube: String,
    pub soundcloud: String,
    pub spotify: String,
}

#[derive(Deserialize)]
pub struct Plural {
    pub tracks: PluralForms,
}

#[derive(Deserialize)]
pub struct PluralForms {
    one: Option<String>,
    few: Option<String>,
    many: Option<String>,
    other: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tui {
    pub window_title: String,
    pub window_hint: String,
    pub window_action: String,
}

impl Dict {
    pub fn load(locale: &str) -> Dict {
        let (locale, source) = match locale {
            "ru" => ("ru", RU),
            "uk" => ("uk", UK),
            _ => ("en", EN),
        };
        // The JSON is compiled in and covered by tests, so a parse failure is a build defect.
        let mut dict: Dict = serde_json::from_str(source).expect("bundled dictionary is valid");
        dict.locale = locale;
        dict
    }

    pub fn locale(&self) -> &'static str {
        self.locale
    }

    pub fn tracks(&self, count: usize) -> String {
        let forms = &self.plural.tracks;
        let form = match plural_rule(self.locale, count) {
            Rule::One => forms.one.as_ref(),
            Rule::Few => forms.few.as_ref(),
            Rule::Many => forms.many.as_ref(),
            Rule::Other => None,
        };
        format(form.unwrap_or(&forms.other), &[("count", &count.to_string())])
    }

    pub fn provider(&self, provider: crate::model::Provider) -> &str {
        use crate::model::Provider;
        match provider {
            Provider::Youtube => &self.providers.youtube,
            Provider::Soundcloud => &self.providers.soundcloud,
            Provider::Spotify => &self.providers.spotify,
        }
    }
}

enum Rule {
    One,
    Few,
    Many,
    Other,
}

/// CLDR cardinal rules for whole numbers, as `Intl.PluralRules` applies them in the window.
fn plural_rule(locale: &str, count: usize) -> Rule {
    let (last, last_two) = (count % 10, count % 100);
    match locale {
        "ru" | "uk" if last == 1 && last_two != 11 => Rule::One,
        "ru" | "uk" if (2..=4).contains(&last) && !(12..=14).contains(&last_two) => Rule::Few,
        "ru" | "uk" => Rule::Many,
        _ if count == 1 => Rule::One,
        _ => Rule::Other,
    }
}

/// Fills `{name}` placeholders, like the window's `format`.
pub fn format(template: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(template.to_owned(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dictionary_parses() {
        for (locale, _) in LOCALES {
            assert_eq!(Dict::load(locale).locale(), locale);
        }
    }

    #[test]
    fn plurals_follow_each_language() {
        let ru = Dict::load("ru");
        assert_eq!(ru.tracks(1), "1 трек");
        assert_eq!(ru.tracks(3), "3 трека");
        assert_eq!(ru.tracks(11), "11 треков");
        assert_eq!(ru.tracks(21), "21 трек");
        assert_eq!(Dict::load("en").tracks(2), "2 tracks");
    }
}
