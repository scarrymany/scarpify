//! Finds a Windows Terminal to host the terminal mode, or fetches Microsoft's portable build,
//! so Windows 10 gets the same colors, emoji and Sixel artwork as Windows 11.

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::process::Command;

use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const RELEASE_URL: &str = "https://api.github.com/repos/microsoft/terminal/releases/latest";
const API_USER_AGENT: &str = "SCARPIFY";
const PORTABLE_DIR: &str = "windows-terminal";
const PARTIAL_SUFFIX: &str = ".partial";
const EXECUTABLE: &str = "WindowsTerminal.exe";
const ALIAS: &str = "wt.exe";
/// Portable builds keep their settings next to the executable when this file exists.
const PORTABLE_MARKER: &str = ".portable";
const SETTINGS_DIR: &str = "settings";
const SETTINGS_FILE: &str = "settings.json";
/// Blends the terminal's padding into the interface background.
const PORTABLE_SETTINGS: &str = include_str!("tui/terminal-settings.json");
const WINDOW_SIZE: &str = "140,42";
const SHA256_PREFIX: &str = "sha256:";

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("no Windows Terminal build for this processor")]
    NoAsset,
    #[error("the downloaded archive is damaged")]
    Checksum,
}

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

/// An installed Windows Terminal first, then a portable one downloaded earlier.
pub fn find(cache_dir: &Path) -> Option<PathBuf> {
    on_path(ALIAS).or_else(|| portable(cache_dir))
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn portable(cache_dir: &Path) -> Option<PathBuf> {
    let executable = cache_dir.join(PORTABLE_DIR).join(EXECUTABLE);
    executable.is_file().then_some(executable)
}

fn asset_suffix() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("_x64.zip"),
        "aarch64" => Some("_arm64.zip"),
        "x86" => Some("_x86.zip"),
        _ => None,
    }
}

/// Downloads the latest portable build from Microsoft's GitHub releases and unpacks it.
pub async fn install(http: &Client, cache_dir: &Path) -> Result<PathBuf, HostError> {
    let suffix = asset_suffix().ok_or(HostError::NoAsset)?;
    let release: Release = http
        .get(RELEASE_URL)
        .header(USER_AGENT, API_USER_AGENT)
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.starts_with("Microsoft.WindowsTerminal_") && asset.name.ends_with(suffix))
        .ok_or(HostError::NoAsset)?;

    let archive = http
        .get(&asset.browser_download_url)
        .header(USER_AGENT, API_USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if let Some(expected) = asset.digest.as_deref().and_then(|d| d.strip_prefix(SHA256_PREFIX)) {
        let actual: String = Sha256::digest(&archive).iter().map(|b| format!("{b:02x}")).collect();
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(HostError::Checksum);
        }
    }

    let cache_dir = cache_dir.to_owned();
    tauri::async_runtime::spawn_blocking(move || unpack(&archive, &cache_dir))
        .await
        .map_err(|e| HostError::Io(io::Error::other(e)))?
}

/// Unpacks next to the final folder and renames it, so an interrupted install is never used.
fn unpack(archive: &[u8], cache_dir: &Path) -> Result<PathBuf, HostError> {
    let target = cache_dir.join(PORTABLE_DIR);
    let partial = cache_dir.join(format!("{PORTABLE_DIR}{PARTIAL_SUFFIX}"));
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    zip::ZipArchive::new(Cursor::new(archive))?.extract(&partial)?;

    // The archive wraps everything in a versioned folder, e.g. `terminal-1.25.2733.0`.
    let root = fs::read_dir(&partial)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join(EXECUTABLE).is_file())
        .unwrap_or_else(|| partial.clone());
    fs::write(root.join(PORTABLE_MARKER), b"")?;
    let settings_dir = root.join(SETTINGS_DIR);
    fs::create_dir_all(&settings_dir)?;
    fs::write(settings_dir.join(SETTINGS_FILE), PORTABLE_SETTINGS)?;

    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::rename(&root, &target)?;
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    Ok(target.join(EXECUTABLE))
}

/// Opens a Windows Terminal window running `executable --tui --hosted`.
///
/// The app is a GUI-subsystem program, which a terminal does not attach to a console by
/// itself; `cmd /c start /wait` gives it a console parent to attach to and keeps the tab
/// open exactly as long as the player runs.
pub fn launch(terminal: &Path, executable: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;

    let (Some(folder), Some(name)) = (executable.parent(), executable.file_name()) else {
        return Err(io::Error::other("unexpected executable path"));
    };
    // Windows Terminal strips the quotes from the command it runs, so a path with spaces
    // would break `start`. The folder goes through `-d`, which it quotes properly, and
    // `start` gets the bare file name, found in that working folder.
    Command::new(terminal)
        // No --tabColor: it would lock the tab color, which follows the accent at runtime.
        .args(["-w", "new", "--size", WINDOW_SIZE, "--title", "SCARPIFY", "-d"])
        .arg(folder)
        .raw_arg(format!("cmd /c start /b /wait {} --tui --hosted", name.to_string_lossy()))
        .spawn()
        .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "downloads Windows Terminal from GitHub"]
    fn installs_the_portable_build() {
        let dir = std::env::temp_dir().join("scarpify-terminal-live");
        let executable = tauri::async_runtime::block_on(install(&Client::new(), &dir)).unwrap();
        println!("{}", executable.display());
        assert_eq!(portable(&dir), Some(executable));
    }

    #[test]
    fn portable_settings_are_valid_json() {
        let settings: serde_json::Value = serde_json::from_str(PORTABLE_SETTINGS).unwrap();
        assert!(settings["schemes"].is_array());
    }

    #[test]
    fn unpacks_into_a_portable_folder() {
        use std::io::Write;

        let mut buffer = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buffer);
        writer
            .start_file("terminal-1.0/WindowsTerminal.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"MZ").unwrap();
        writer.finish().unwrap();

        let dir = std::env::temp_dir().join(format!("scarpify-host-{}", std::process::id()));
        let executable = unpack(buffer.get_ref(), &dir).unwrap();
        assert_eq!(executable, dir.join(PORTABLE_DIR).join(EXECUTABLE));
        assert!(dir.join(PORTABLE_DIR).join(PORTABLE_MARKER).is_file());
        assert_eq!(portable(&dir), Some(executable));
        fs::remove_dir_all(dir).unwrap();
    }
}
