<p align="center"><img src="app-icon.svg" width="96" alt="Hotswap logo"></p>

# Hotswap

A desktop app that switches between git worktrees of projects whose API and front end live in separate repositories: one click stops the old pair's dev servers and starts the new pair, each in its own terminal.

## About

If you work on several branches at once, switching usually means opening terminals, `cd`-ing into two worktrees, killing the old `npm run dev` processes, waiting for the ports to free up and starting everything again. Hotswap does that in one step.

It pairs the API and front worktrees by branch name and keeps **one worktree running per project**. When you switch, it sends Ctrl+C to the old pair, waits until every process in the group has exited (killing them after a timeout), and only then starts the new pair. Terminal multiplexers and editor terminals don't know about the pairing or this lifecycle.

> The interface is in Brazilian Portuguese.

## Features

- **Paired worktrees.** API and front worktrees are matched by branch name, with manual pairs for branches named differently. A worktree with no match shows up alone.
- **Real terminals.** Each side gets a real PTY (xterm.js) running your own shell, with clickable links and search.
- **Safe switching.** You can see each worktree's state at a glance (running, starting, stopping, crashed), with "Force now" and "Go back" while the old pair stops.
- **Parallel mode**, per project. It runs several worktrees side by side, gives each one its own ports through environment variables, and points each front end at its own API.
- **Worktree management.** Create worktrees (also from an open PR), rename, move, remove and `git pull` them from the app.
- **Projects from the UI.** Point to local folders, or clone from GitHub with `gh` into a bare-repo layout.
- **Keyboard first.** A command palette (`Ctrl+Shift+K`) and a shortcut for every action.
- **Lives in the tray.** Closing the window keeps the dev servers running.

## Installation

Download the file for your system from the [latest release](https://github.com/martinsgabriel1956/hotswap/releases/latest):

| System | File | Install |
|--------|------|---------|
| Debian / Ubuntu | `hotswap_<version>_amd64.deb` | `sudo apt install ./hotswap_<version>_amd64.deb` |
| Fedora | `hotswap-<version>-1.x86_64.rpm` | `sudo dnf install ./hotswap-<version>-1.x86_64.rpm` |
| Arch | `PKGBUILD` | `makepkg -si` in the folder with the PKGBUILD |
| Other Linux | `hotswap_<version>_amd64.AppImage` | `chmod +x` it and run it |
| Windows | `hotswap_<version>_x64-setup.exe` | Run the installer |
| macOS | `hotswap_<version>_universal.dmg` | Drag to Applications, then run `xattr -cr /Applications/hotswap.app` |

The macOS app isn't signed by Apple. Without the `xattr` command, macOS reports it as damaged.

Hotswap is built and used daily on Linux. The Windows and macOS builds compile in CI but haven't been used on those systems.

### Requirements

- `git`
- [`gh`](https://cli.github.com/) (optional), logged in with `gh auth login`. You need it to clone from GitHub, create worktrees from PRs and open a branch's PR.

## Usage

1. Open Hotswap and click **Criar projeto** (create project).
2. Choose the API and front repositories (local folders, or clone them from GitHub), and set the start command for each side (default `npm run dev`).
3. Pick a worktree in the sidebar or the palette (`Ctrl+Shift+K`). It starts with one terminal for the API and one for the front end. Picking another worktree of the same project stops the current one first.

The config is saved in `~/.config/hotswap/config.json` (`%USERPROFILE%\.config\hotswap\config.json` on Windows). Some settings are only edited by hand:

```json
{
  "stopTimeoutMs": 5000,
  "editorCommand": "code",
  "projects": [
    {
      "name": "shop",
      "api":   { "repo": "~/dev/shop-api", "startCommand": "pnpm dev" },
      "front": { "repo": "~/dev/shop-web", "startCommand": "pnpm dev" },
      "manualPairs": [{ "api": "feat/x", "front": "feature/x" }],
      "baseBranch": "main"
    }
  ]
}
```

To use parallel mode, turn on the switch next to the project name and tell Hotswap which environment variables carry the ports:

```json
"api":   { "parallelEnv": { "PORT": "{apiPort}" } },
"front": { "parallelEnv": { "PORT": "{frontPort}", "NEXT_PUBLIC_API_URL": "http://localhost:{apiPort}" } }
```

[`SPEC.md`](SPEC.md) describes every field and behavior (in Portuguese).

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+K` | Palette: search and switch worktrees |
| `Ctrl+Shift+J` | Toggle between the API and Front tabs |
| `Ctrl+Shift+X` / `Ctrl+Shift+S` | Stop / start the open tab (add `Alt` for both sides) |
| `Ctrl+Shift+R` | Restart the open tab (add `Alt` for both sides) |
| `Ctrl+Shift+N` | New worktree |
| `Ctrl+Shift+F` | Search the visible terminal |
| `Ctrl+Shift+O` | Open the tab's folder in your editor |
| `Ctrl+Shift+L` | Reload the worktree list |
| `Ctrl+Shift+B` / `Ctrl+Shift+E` | Collapse / focus the sidebar |
| `Ctrl+Shift+H` | Help: every shortcut and the state legend |

## Development

Hotswap is built with [Tauri v2](https://v2.tauri.app/): a Rust backend (`portable-pty`) and a plain TypeScript + Vite front end, with no framework. You need Node.js, Rust and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your system. On Debian/Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

```bash
git clone https://github.com/martinsgabriel1956/hotswap.git
```

```bash
cd hotswap && npm install
```

```bash
npm run tauri dev
```

Run the tests:

```bash
npm test
```

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Build the Linux packages:

```bash
npm run tauri build -- --bundles deb,rpm,appimage
```

Releases are published by [`scripts/release.sh`](scripts/release.sh), from CI or by hand.

## License

Hotswap is licensed under the MIT license. See [`LICENSE`](LICENSE).
