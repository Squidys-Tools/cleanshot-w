# Distribution and Release

## Packaging

Tauri builds the frontend into the binary and ships `tessdata/` via the
`resources` config. Two packaging options, both no-admin for the end user:

```
Option A — portable ZIP                     Option B — per-user NSIS
CleanShotW-0.1.0-win64.zip                  CleanShotW-0.1.0-win64-setup.exe
  CleanShotW.exe                              (tauri nsis installMode: currentUser)
  resources/  (tessdata, etc.)
  CleanShotW-0.1.0-win64.zip.sha256
```

## Release checklist

Every release includes:

1. Binary artifact (ZIP or NSIS)
2. SHA-256 checksum sidecar
3. Release notes in `CHANGELOG.md`
4. Git tag

## Distribution notes

- SmartScreen will flag unsigned binaries — document "More info → Run anyway"
  in the release notes.
- No auto-update in v1.
- No code signing in v1.
- Future: optional update checker (compare version file over HTTPS,
  user-downloaded ZIP).

## Target directory structure on user machines

```
%LOCALAPPDATA%\CleanShotW\
  library\<uuid>\image.png
                 annotations.json
                 thumbnail.png
  settings.json
  WebView2\                (user data folder)
  logs\
```
