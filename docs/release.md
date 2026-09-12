# Release

The Windows release package contains `hashoverlay.exe`, the Settings application, the default configuration path created on first use, and the short user guide. End users should not need Rust, Cargo, Node, npm or an editor.

Before publishing, run the validation commands in [development.md](development.md), verify the Settings icon/bundle, and check the overlay over both a bright and a dark LMU scene. The release workflow stages the overlay executable, Settings bundle, README, license and usage guide into one ZIP.
