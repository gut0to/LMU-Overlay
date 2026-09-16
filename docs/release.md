# Release

The Windows release contains the standalone `hashoverlay.exe`, the
`HashOverlay Settings` installer, the matching Settings resource bundle, the
README, license and usage guide. End users should not need Rust, Cargo, Node,
npm or an editor.

## Release Checklist

1. Update the application version consistently in every release manifest.
2. Run the commands in [development.md](development.md), including the
   Settings shell tests and strict Clippy check.
3. Build the release overlay before building the Tauri app so the installer
   embeds the matching `hashoverlay.exe`.
4. Verify the NSIS installer and release ZIP contain the expected executable,
   README, license and usage guide.
5. Run [live-smoke-test.md](live-smoke-test.md) on a Windows machine with LMU
   before claiming live-game validation.
6. Merge the release branch into `main`, create an annotated `vX.Y.Z` tag on
   that merge commit and push the tag.

Pushing a version tag starts the Windows release workflow. It publishes
`HashOverlay-overlay-x64.zip` and `HashOverlay-Setup-x64.exe` on GitHub
Releases. Do not retag an existing release; publish a patch version for a
corrected package instead.
