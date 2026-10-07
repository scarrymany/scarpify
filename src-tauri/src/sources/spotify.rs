//! Account-free Spotify import: reads public playlists and albums from the embed player page.

use futures_util::{stream, StreamExt};
use reqwest::header::USER_AGENT;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::BROWSER_USER_AGENT;
use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, Provider, Track};

const EMBED_URL: &str = "https://open.spotify.com/embed";
const OPEN_HOST: &str = "open.spotify.com";
const URI_PREFIX: &str = "spotify:";
const NEXT_DATA_START: &str = r#"<script id="__NEXT_DATA__" type="application/json">"#;
const NEXT_DATA_END: &str = "</script>";
const ENTITY_POINTER: &str = "/props/pageProps/state/data/entity";
const OEMBED_URL: &str = "https://open.spotify.com/oembed";
/// Parallel cover lookups: fast for 100-track playlists without hammering the endpoint.
const COVER_CONCURRENCY: usize = 8;

#[derive(Deserialize)]
struct OEmbed {
    thumbnail_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct Link<'a> {
    kind: &'a str,
    id: &'a str,
}

pub async fn import(http: &Client, link: &str) -> Result<Collection> {
    let link = parse_link(link).ok_or(Error::UnsupportedLink)?;
    let page = http
        .get(format!("{EMBED_URL}/{}/{}", link.kind, link.id))
        .header(USER_AGENT, BROWSER_USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let data = page
        .split_once(NEXT_DATA_START)
        .and_then(|(_, rest)| rest.split_once(NEXT_DATA_END))
        .map(|(json, _)| json)
        .ok_or_else(|| Error::Unavailable("Spotify page has no embedded data".into()))?;
    let root: Value = serde_json::from_str(data)?;
    let entity = root
        .pointer(ENTITY_POINTER)
        .ok_or_else(|| Error::Unavailable("playlist is private or does not exist".into()))?;

    let mut collection = parse_entity(entity)?;
    // Playlist embeds carry only the playlist cover; albums share one cover anyway.
    if link.kind == "playlist" {
        fill_track_covers(http, &mut collection.tracks).await;
    }
    Ok(collection)
}

async fn fill_track_covers(http: &Client, tracks: &mut [Track]) {
    // Owned ids keep the future free of borrowed closures, which `Send` inference rejects.
    let uris: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
    let covers: Vec<Option<String>> = stream::iter(uris)
        .map(|uri| {
            let http = http.clone();
            async move { track_cover(&http, &uri).await }
        })
        .buffered(COVER_CONCURRENCY)
        .collect()
        .await;
    for (track, cover) in tracks.iter_mut().zip(covers) {
        if cover.is_some() {
            track.artwork = cover;
        }
    }
}

/// A missing cover is cosmetic, so any failure just keeps the playlist artwork.
async fn track_cover(http: &Client, uri: &str) -> Option<String> {
    let response = http
        .get(OEMBED_URL)
        .header(USER_AGENT, BROWSER_USER_AGENT)
        .query(&[("url", uri)])
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    response.json::<OEmbed>().await.ok()?.thumbnail_url
}

fn parse_link(link: &str) -> Option<Link<'_>> {
    let link = link.trim();
    let mut parts: Vec<&str> = if let Some(uri) = link.strip_prefix(URI_PREFIX) {
        uri.split(':').collect()
    } else {
        let path = link
            .split_once(OPEN_HOST)?
            .1
            .split(['?', '#'])
            .next()?;
        path.split('/').filter(|p| !p.is_empty()).collect()
    };

    // Localized links look like /intl-de/playlist/<id>.
    if parts.first().is_some_and(|p| p.starts_with("intl-")) {
        parts.remove(0);
    }
    match parts.as_slice() {
        [kind @ ("playlist" | "album"), id, ..]
            if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            Some(Link { kind, id })
        }
        _ => None,
    }
}

fn parse_entity(entity: &Value) -> Result<Collection> {
    let str_at = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);

    let artwork = entity
        .pointer("/coverArt/sources/0/url")
        .or_else(|| entity.pointer("/visualIdentity/image/0/url"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let is_album = entity.get("type").and_then(Value::as_str) == Some("album");
    let album = is_album.then(|| str_at(entity, "name")).flatten();

    let tracks = entity
        .get("trackList")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Unavailable("no tracks found".into()))?
        .iter()
        .filter(|t| t.get("isPlayable").and_then(Value::as_bool) != Some(false))
        .filter_map(|t| {
            Some(Track {
                id: str_at(t, "uri")?,
                provider: Provider::Spotify,
                title: str_at(t, "title")?,
                artists: str_at(t, "subtitle")
                    .map(|s| s.split(',').map(|a| a.trim().to_owned()).collect())
                    .unwrap_or_default(),
                album: album.clone(),
                duration_ms: t.get("duration").and_then(Value::as_u64),
                artwork: artwork.clone(),
                url: None,
            })
        })
        .collect();

    Ok(Collection {
        kind: if is_album { CollectionKind::Album } else { CollectionKind::Playlist },
        name: str_at(entity, "name").unwrap_or_default(),
        owner: str_at(entity, "subtitle"),
        artwork,
        provider: Provider::Spotify,
        tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_links() {
        let expected = Some(Link { kind: "playlist", id: "37i9dQZF1DXcBWIGoYBM5M" });
        assert_eq!(parse_link("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=abc"), expected);
        assert_eq!(parse_link("https://open.spotify.com/intl-de/playlist/37i9dQZF1DXcBWIGoYBM5M"), expected);
        assert_eq!(parse_link("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"), expected);
        assert_eq!(
            parse_link("https://open.spotify.com/album/4aawyAB9vmqN3uQ7FjRGTy"),
            Some(Link { kind: "album", id: "4aawyAB9vmqN3uQ7FjRGTy" })
        );
        assert_eq!(parse_link("https://open.spotify.com/track/11hcBLPtbMp4aQI6zGQLub"), None);
        assert_eq!(parse_link("https://example.com/playlist/x"), None);
    }
}
