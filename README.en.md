# HomeDesktop

A lightweight cross-platform (Windows / macOS / Linux) desktop launcher that manages "icons" the way a phone home screen does — apps, folders, and **plugin widgets** (clock / weather / calendar / system monitor / todo / Home Assistant) mixed on one screen, in full-screen Pad mode with global hotkeys, plus a local plugin market and zip distribution.

> Tech stack: **Tauri 2** (Rust) + **Svelte 5** (TypeScript / Vite 6)

![Main UI](docs/screenshot-main.png)

## ✨ Features

- **Phone-style desktop**: icons + widgets mixed on screen, multi-page swipe, folders, global search (`Ctrl+Space`)
- **Drag & drop**: drag to reorder (mouse / long-press on touch), drag into folders, drag to edge to flip pages, cross-page moves
- **Sizing**: integer multiples of the 1×1 base (⇲ button), widgets offer the size grid declared by their plugin
- **Full-screen Pad**: `Alt+Q` global hotkey to show / hide; immersive full-screen launch pad
- **Appearance**: light / dark theme, background presets / custom color / local image wallpaper, icon size, **grid spacing slider**
- **System tray**: single-click show / hide, quick menu; **launch on boot** toggle
- **System apps panel**: scan registry + Start menu, show real system icons, one click to place them on the desktop
- **Plugin system v2**:
  - zip plugin packages (root `manifest.json`); in-app ＋ → install / update / uninstall
  - **Custom JS widgets**: plugin bundle `widget.js` custom elements run in an **iframe sandbox** (`allow-scripts`, no same-origin), bridged via `window.__homedesktopPlugin` postMessage for settings & capabilities
  - **Plugin notifications**: `bridge.notify(title, body)` → system toast (example: countdown timer)
  - **Local plugin market**: scan a market directory of zip packages, one-click install; unified manage page (installed / market tabs)
  - **Provider system**: one provider can ship multiple sub-plugins (e.g., Home Assistant → 💡 Light / 🔌 Switch / 📊 Sensor) shown as a two-level menu with shared provider-level config
- **Built-in widgets**: clock, weather (multi-city instances), calendar (with lunar dates), system monitor (CPU / memory), todo, music (with lyrics), **Home Assistant** (state & control for light / switch / sensor entities)
- **Home Assistant integration**:
  - Auto-fetch entities in the settings menu, check to add them
  - MDI icon picker (preset grid + custom icon name) and custom display names per entity
  - Large circular status icon: amber (FFC107) when on, gray when off; tap to toggle
- **Data**: local persistence of layout / config (`layout.json` / `config.json`), **export / import backup** (overwrite / merge)

## 📖 Documentation

| Document | Contents |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Product requirements |
| [docs/tech-selection.md](docs/tech-selection.md) | Technology choices |
| [docs/architecture.md](docs/architecture.md) | Architecture |
| [docs/milestones.md](docs/milestones.md) | Milestones (M0–M17 all done ✅) |
| [docs/PLUGIN_API.md](docs/PLUGIN_API.md) | **Plugin developer API** (manifest schema / widgets / bridge / distribution) |
| [docs/CHANGELOG.md](docs/CHANGELOG.md) | Changelog (long-term memory) |

## 🚀 Development

```bash
# Install dependencies (Node ≥ 20.17; on Node 24 use corepack pnpm)
corepack pnpm install --store-dir .pnpm-store

# Dev run (starts Vite + the Tauri window)
corepack pnpm tauri dev

# Frontend type-check / tests
corepack pnpm check
corepack pnpm test
```

Directory layout:

```
src/            Frontend (Svelte 5)
src/core/       Core: types / state / persistence / plugin loader / widget runtime
src/components/ Grid / icon tiles / search / add menu / plugin manager
src/widgets/    Built-in widgets (clock / weather / calendar / system monitor / todo / music / HA)
src-tauri/      Rust shell (commands / plugin registry / layout persistence / notifications)
crates/         homedesktop-core (pure logic core, unit-testable)
plugins/        Built-in plugins (one dir + manifest.json each)
plugins-dist/   Sample plugin zip packages
docs/           Product & technical docs
```

## 🔌 Plugin development

Each plugin = a directory + `manifest.json`:

```json
{
  "id": "com.example.myplugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "type": "widget",
  "emoji": "🧩",
  "widgetComponent": "__plugin__",
  "widgetFile": "widget.js",
  "widgetElement": "hd-my-widget",
  "sizes": [{ "w": 2, "h": 1 }],
  "settings": [
    { "key": "target", "label": "Target seconds", "type": "number", "default": 60 },
    { "key": "enableNotify", "label": "Notify on completion", "type": "toggle", "default": true }
  ],
  "actions": []
}
```

- Bridge API available in widget JS: `getSetting(cellId, key, fallback?)` / `setSetting(cellId, key, value)` / **`notify(title, body)`**
- Distribution: zip the directory (root contains `manifest.json`) → in-app ＋ → install from zip; or drop into the local market directory
- Full spec in [docs/PLUGIN_API.md](docs/PLUGIN_API.md); sample packages in `plugins-dist/`

## 📦 Packaging

```bash
corepack pnpm tauri build
```

| Artifact | Size (Windows, measured) |
|---|---|
| NSIS installer `HomeDesktop_*-setup.exe` | ~1.1 MB |
| MSI installer | ~1.7 MB |
| Portable `homedesktop.exe` | ~3.2 MB |

- All three platforms (Windows / macOS / Linux) are built by [.github/workflows/build.yml](.github/workflows/build.yml) via GitHub Actions
- Windows release builds statically import `WebView2Loader.dll`; the CI pulls it from NuGet before building; the NSIS hook (`nsis-hooks.nsh`) copies it next to the exe during install

## 📄 License

[MIT](LICENSE) © 2026 Eggplant4363
