# Platform architecture

One Tauri 2 application shares the Svelte interface, typed IPC commands, Rust
provider clients, streaming decoder, and SQLite library. Data remains under
Tauri's app-data directory and caches under its cache directory; no storage
permission or Windows path is required.

| Component | Windows / Linux | Android |
| --- | --- | --- |
| Search, imports, SQLite library | Shared Rust commands | Same commands and private app storage |
| Streaming and decoding | rodio / CPAL native output | CPAL AAudio (requires native Android integration) |
| TLS | Rustls with WebPKI roots | Same; avoids cross-compiling OpenSSL |
| Window buttons | Tauri desktop permissions | Not exposed; Android owns windows |
| Output selection | CPAL enumerated devices | OS routes audio; UI explains restriction |
| Discord presence | Local desktop IPC | Unavailable: no desktop Discord IPC service |
| Settings / playlists | Existing Svelte state and SQLite | Shared, responsive shell required |

The UI capability helper controls presentation only. Tauri capability files
restrict desktop commands to desktop targets, independently of user-agent
strings. Android keeps compatible presence commands as no-ops so shared player
state does not need to fork; the settings UI explicitly explains this limitation.

Rustls uses public WebPKI roots rather than the OS trust store. Locally installed
enterprise roots are not automatically trusted. Certificate verification remains
enabled. RustyPipe must also disable its default native-TLS feature, otherwise
Cargo feature unification would silently retain the OpenSSL dependency.

Target-specific native lifecycle, packaging and smoke tests are separate changes.
Compilation alone does not establish playback or lifecycle compatibility.
