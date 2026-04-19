# AkTags

AI-powered tag-based file browser with a native GUI, built in Rust.

## Features

- **AI-Powered Tagging** - Automatically tags files using Ollama LLM
- **Full-Text Search** - Fast FTS5 search across all file content and tags
- **Real-time Monitoring** - Watches directories and processes new files automatically
- **Multi-Theme Support** - Light, Dark, and Eldritch themes
- **Taxonomy Management** - Pending tag queue and approved tag library

## Requirements

- **Ollama** - Local LLM server (or remote URL configurable)
- **Linux** - Built for Wayland/X11

## Installation

### From Release
Download the latest release from the [GitHub Releases](https://github.com/Akinus21/Aktags/releases) page.

```bash
chmod +x aktags
./aktags
```

### From Source
```bash
cargo build --release
./target/release/aktags
```

## Command Line Options

```bash
aktags           Start GUI with embedded daemon (default)
aktags --daemon Start daemon-only mode (no GUI)
aktags --help   Show help
```

## Configuration

On first launch, you'll configure:

1. **Ollama Base URL** - Your Ollama server endpoint
2. **Model** - The LLM model to use for tagging
3. **Watch Directory** - Directory to monitor for file changes

## Autostart

### Daemon Only (systemd user service)

Run the background daemon on login without the GUI:

```bash
# Install the systemd user service
mkdir -p ~/.config/systemd/user
cp aktags-daemon.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable aktags-daemon
systemctl --user start aktags-daemon

# Check status
systemctl --user status aktags-daemon

# View logs
journalctl --user -u aktags-daemon
```

### GUI on Login (desktop autostart)

Launch the full GUI application on login:

```bash
# Install desktop autostart
mkdir -p ~/.config/autostart
cp aktags.desktop ~/.config/autostart/
```

Note: The desktop autostart launches the full GUI application. If you only want the daemon running, use the systemd service above instead.

### Install Binary System-wide

To install the binary for all users:

```bash
sudo cp aktags /usr/local/bin/
sudo chmod +x /usr/local/bin/aktags

# Then for autostart:
cp aktags-daemon.service ~/.config/systemd/user/
cp aktags.desktop ~/.config/autostart/
```

## Architecture

- **GUI**: iced 0.13 (pure Rust, Elm-style)
- **Backend Daemon**: Tokio-based file watcher and tagger
- **Database**: SQLite with FTS5 full-text search
- **LLM**: Ollama HTTP client

## Keyboard Shortcuts

- `Enter` in search - Submit search
- Click file - Select and view details
- Click tag - Toggle filter

## Building

```bash
cargo build --release
```

Requires GTK3 and related libraries:
```bash
sudo apt install libgtk-3-dev libxkbcommon-dev libwayland-dev libxkbcommon-x11-dev pkg-config
```

## License

MIT