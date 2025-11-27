[<img src="./media/laio.svg" width="450" />](https://laio.sh)

A simple, flexbox-inspired layout and session manager for tmux. Define complex multi-window, multi-pane layouts in YAML and manage session lifecycles with startup/shutdown hooks.

**[Full Documentation](https://laio.sh/docs/getting-started/installing)**

## Features

- **Flexbox-inspired layouts** - Define pane splits with `flex` and `flex_direction` (row/column)
- **Session lifecycle management** - Startup/shutdown commands and embedded scripts
- **Dual configuration modes** - Global configs (`~/.config/laio`) or local project configs (`.laio.yaml`)
- **Session serialization** - Export existing tmux sessions to YAML format
- **Focus & zoom control** - Pin focus and zoom states in configuration
- **Environment & shell customization** - Per-session env vars and shell overrides
- **Tmux native** - Built for tmux (experimental Zellij support available)

## Quick Start

```bash
# Install (see Installation section)
nix profile install "github:ck3mp3r/laio-cli"

# Create a new config
laio config create myproject

# Start the session
laio start myproject

# List sessions and configs
laio list
```

## Installation

Supported platforms: Linux and macOS (aarch64, x86_64)

### Nix

[![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)
```bash
nix profile install "github:ck3mp3r/laio-cli"
```

### Homebrew

```bash
brew tap ck3mp3r/laio-cli https://github.com/ck3mp3r/laio-cli/
brew install laio
```

### Binary Download

Download from the [Release Page](https://github.com/ck3mp3r/laio-cli/releases) and add to `PATH`.

## Development

### Prerequisites

- Rust 1.70+ (or use Nix for reproducible builds)
- tmux 3.0+

### Build

```bash
cargo build --release
# Binary at: target/release/laio
```

### Test

```bash
cargo test
```

### Development Environment (Nix)

```bash
nix develop
# or use direnv
direnv allow
```

### Configuration Schema

The YAML configuration schema is defined in `src/common/config/schema.json`. Use this for editor integration (LSP, validation).

## Usage

### Commands

```bash
laio start [name]              # Start session (interactive picker if name omitted)
laio start --file config.yaml  # Start from specific file
laio stop [name]               # Stop session
laio stop --all                # Stop all laio-managed sessions
laio list                      # List sessions and configs
laio config create <name>      # Create new config
laio config create --copy src  # Create from existing config
laio config edit <name>        # Edit config in $EDITOR
laio config link <name>        # Symlink .laio.yaml to global config
laio session yaml              # Export current tmux session to YAML
laio completion <shell>        # Generate shell completions
```

See `laio --help` or [full documentation](https://laio.sh/docs/getting-started/basics) for all options.

### Configuration

Configurations are YAML files defining session layouts:

```yaml
name: myproject
path: /path/to/myproject

# Session lifecycle hooks
startup:
  - command: docker-compose
    args: [up, -d]

startup_script: |
  #!/usr/bin/env bash
  echo "Session starting..."

shutdown:
  - command: docker-compose
    args: [down]

shutdown_script: |
  #!/usr/bin/env bash
  echo "Cleaning up..."

# Session environment
shell: /bin/zsh
env:
  NODE_ENV: development
  DEBUG: "app:*"

# Window and pane layout
windows:
  - name: editor
    panes:
      - name: nvim
        commands:
          - command: $EDITOR

  - name: dev
    flex_direction: row  # horizontal splits (panes side-by-side)
    panes:
      - flex: 2
        flex_direction: column  # vertical splits (panes stacked)
        panes:
          - flex: 3
            path: ./src
            focus: true  # initial focus
            commands:
              - command: npm
                args: [run, dev]
          - flex: 1
            zoom: true  # start zoomed
            script: |
              #!/usr/bin/env bash
              tail -f logs/app.log
      - flex: 1
        style: bg=blue,fg=white  # tmux pane styling
```

**Key features:**
- `flex`: Proportional sizing (e.g., `flex: 2` = twice the size of `flex: 1`)
- `flex_direction`: `row` (horizontal) or `column` (vertical)
- `focus`: Pin initial cursor position
- `zoom`: Start pane in zoomed state
- `commands`: Sequential command execution
- `script`: Inline script blocks
- `path`: Working directory (absolute or relative to session root)

See [configuration docs](https://laio.sh/docs/getting-started/basics#configuration-yaml) for all options.

## Project Configurations

Create `.laio.yaml` in any project directory:

```bash
cd myproject
laio config create  # creates .laio.yaml
laio start          # auto-detects and uses .laio.yaml
```

Link local configs to global namespace:

```bash
laio config link myproject  # symlinks .laio.yaml -> ~/.config/laio/myproject.yaml
```

## Session Serialization

Export existing tmux sessions to YAML:

```bash
# From within a tmux session
laio session yaml > ~/.config/laio/newsession.yaml
```

## Usage Restrictions

Use of this software is strictly prohibited for any organization, company, or government directly or indirectly involved in aiding or abetting the genocide and atrocities committed against the Palestinian people in Gaza. Only individuals and entities unaffiliated with such actions are permitted to use the software.

## License

See [LICENSE.md](LICENSE.md)
