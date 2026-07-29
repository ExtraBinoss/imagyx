# Updates

Imagyx uses signed Tauri updater artifacts published as GitHub Releases.

Before the first release, generate an updater signing key with the Tauri CLI. Store its private key and password in the GitHub secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Put the corresponding public key in `src-tauri/tauri.conf.json` before enabling the updater plugin.

Run the `Release` GitHub Actions workflow manually and enter `[RELEASE]`. It validates that `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` use the same new version before it builds the signed Windows NSIS installer.
