# Changelog

This project follows Semantic Versioning. User-visible changes are recorded here.

## [0.2.0] - 2026-10-22

### Added

- Aether core v2.3.0 compatibility
- Built-in Tor profiles with bundled pluggable transport support
- MASQUE identity import into protected app-managed storage
- Tor bridge files and relay discovery controls
- Desktop notifications for connection state changes
- GUI control for automatic replacement of refused identities

### Fixed

- Enabled the Tor feature in the GTK release build
- Included Tor transports in release archives and installation

## [0.1.1] - 2026-09-23

### Added

- Uninstall option in the per-user installer
- Maintainer and release links in Settings
- Light and dark mode screenshots in the README

## [0.1.0] - 2026-09-23

### Added

- GTK desktop app for Aether's nine connection profiles
- Direct integration with Aether's cancellable Rust core API
- Saved appearance and non-secret connection preferences
- Activity view, connection status, and SOCKS5 proxy controls
- Optional GNOME Quick Settings extension and D-Bus control service
- In-app Psiphon client setup
- Psiphon client included with release downloads
- Per-user Linux x86_64 release installer and checksum verification
