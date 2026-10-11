# Android

The Android app uses the shared Svelte interface, SQLite library, Rust providers and
rodio/Symphonia player. CPAL uses AAudio, requiring Android 8/API 26 or later. The
committed Gradle project is in `src-tauri/gen/android`; do not rerun `android init`
over it without reviewing changes. Kotlin integration lives in `src-tauri/android`.

## Toolchain and builds

- Node 22.12+ or 24, stable Rust with `aarch64-linux-android` and `x86_64-linux-android`.
- JDK17, Android SDK platform37, build-tools37, NDK27.2.12479018.
- LLVM/libclang for RustyPipe/QuickJS bindgen (`libclang-dev` on Debian-family build
  hosts). Set `LIBCLANG_PATH` if libclang is not on its standard library path.
- Set `JAVA_HOME`, `ANDROID_HOME`, and `NDK_HOME`; add platform-tools to `PATH`.

```sh
npm ci
npm run check
npm test
npm run android -- build --debug --target aarch64 --apk
npm run android -- build --debug --target x86_64 --apk
```

The wrapper supplies the NDK sysroot and Clang builtin headers to bindgen. The
upstream QuickJS crate has no pre-generated Android bindings; substituting Linux
bindings is not safe. Output APKs are under
`src-tauri/gen/android/app/build/outputs/apk`. Install a debug APK with `adb install -r`.
Debug builds use the standard debug signing key and enable local development HTTP;
release builds keep cleartext disabled. Never publish the debug or smoke-test APK.

For production use `npm run android -- build --target aarch64 --apk` and configure
[Android signing](https://v2.tauri.app/distribute/sign/android/) with a private release
keystore. Do not commit signing keys/passwords. Updates must use the same application
ID and signing key; `adb install -r`/in-place package upgrades preserve app-private data.
Uninstalling removes it. A release build without a configured key is unsigned.

## Interaction and storage

The layout switches to a library drawer and stacked player below 900 CSS pixels;
all views remain shared with desktop. Track menus are available through a visible
more button, nested menus open on tap, and playlist/queue reorder actions use buttons
so vertical gestures continue to scroll. Android Back dismisses menus/dialogs/queue/
drawer, then navigates history, and finally backgrounds the activity.

SQLite and provider caches use Tauri's app-private directories. UI preferences and
the paused queue use app-private WebView storage. No broad filesystem/media read
permission is requested. The system file chooser grants access to a selected cover;
the image is converted and stored in the library. Inputs resize with the keyboard,
modals scroll, and layout accounts for safe areas.

## Audio and lifecycle

Playback runs in Rust; an Android foreground service provides a MediaSession and
persistent notification, system play/pause/next/previous/stop, audio-focus handling,
headphone-disconnect pause and a bounded wake lock renewed during playback. Native
pause/stop/focus controls use a Rust channel, not JavaScript timers. Focus loss pauses;
transient loss may resume only if playback was active. Service-start failures abandon
focus and release the wake lock. Swiping the task away stops playback; cold restart
restores the queue paused instead of unexpectedly playing audio. A paused service
stops after ten minutes. The app does not request battery-optimization exemptions.

Android media-session notifications are exempt from the ordinary notification
permission prompt; the manifest requests foreground-media and wake-lock permissions,
not microphone or arbitrary storage permissions. Android 15+ restricts audio focus
and foreground-service starts from the background; failures are surfaced to the UI.

## Explicit differences and verification boundaries

- Desktop Discord IPC is unavailable on Android; Settings explains it and disables
  that switch. There is no equivalent public local Discord mobile IPC API.
- Android controls default audio routing; CPAL exposes devices available to AAudio.
  Physical Bluetooth/wired hotplug still requires device testing.
- Queue advancement and next/previous use the shared WebView queue. Native media
  pause/focus handling is independent. Background advancement must be verified on
  target OS/OEM builds; an emulator is not proof of behavior under OEM battery policies.
- Process death does not auto-resume audio. The restored queue resumes on user action.
- Signing/Play publication and physical-device checks require the maintainer's key
  and hardware; neither is fabricated by successful compilation.

## Offline native QA

`--features smoke-test` is an opt-in Cargo feature that substitutes synthetic search,
import and twelve-second generated MP3 tones. It exercises the real IPC, SQLite,
decoder, output and service while the emulator/network namespace is offline. Search
any non-empty query to find three tracks. This feature is never enabled in release CI.
The tone source is `ffmpeg -f lavfi -i sine=frequency=440:duration=12 -af volume=0.05
-codec:a libmp3lame -b:a 64k tone.mp3` (synthetic, no third-party recording).

Keep debug fixture APK results separate from production build evidence. Record
search/play/pause/seek/queue, background advancement, focus interruption, restart
persistence, rotation, keyboard, Back and file-picker results in the audit matrix.
