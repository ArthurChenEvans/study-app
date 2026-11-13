# Releasing

Every push to the `release` branch is a new version (0.1.N). GitHub Actions runs the Fedora
update test, builds the signed .rpm and publishes it as a GitHub Release with `latest.json`.
Installed copies of the app find it the next time they open or close.

```bash
git push origin HEAD:release
```

## One-time setup

1. Add two repository secrets (Settings, Secrets and variables, Actions, New repository secret):
   - `TAURI_SIGNING_PRIVATE_KEY`: the whole contents of the private key file
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: leave empty (the key has no password)
2. Keep a copy of the key file somewhere safe, like a password manager. Without it, installed
   copies can't be updated again and everyone has to reinstall by hand.
3. Install the first updater-enabled .rpm by hand, from the newest release:

   ```bash
   sudo dnf install ./study-app-0.1.*.x86_64.rpm
   ```

After that the app updates itself. Installing an update needs your password, because an .rpm
installs for the whole system.

## Tests

- `cargo test -p study-app updates` checks newer, same, older, bad signature and offline
  against a fake update server, with a throwaway key in `src-tauri/tests/fixtures`.
- `.github/workflows/updates-e2e.yml` installs an old build on Fedora, publishes a newer one,
  lets the app update itself and checks the data survives. It also checks a tampered package
  is refused. Release runs it first.
