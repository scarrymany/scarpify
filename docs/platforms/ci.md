# Build and verification gates

Every pull request runs the shared frontend checks/tests and native Rust tests on
Windows and Linux, builds Linux deb/RPM/AppImage packages, and compiles Android
arm64 production and x86_64 offline fixture APKs. Production Android outputs are
unsigned until the maintainer configures a private signing key. Fixture APKs use a
debug key and synthetic audio; never distribute them as production releases.

The API35 emulator job disables networking, installs the fixture APK, verifies that
the native process survives launch, and saves screenshots and logs. This is a launch
smoke test, not proof of playback, audio focus, physical routing or OEM background
behavior. Those require the manual native scenarios listed in android.md. Rust
network retry tests use loopback fixtures; ignored live-provider/audio-hardware tests
remain explicit and do not silently call real accounts in CI.

Artifacts are retained by Actions and are not published automatically. The existing
Windows release workflow remains manually dispatched and creates draft releases.
Linux packages built on Ubuntu24.04 require a compatible glibc/system WebKit runtime;
build on the oldest supported distribution before promising older-distro support.
