# MCP Tools for Laio

MCP (Model Context Protocol) tools for extending functionality through the `nu-mcp` server.

## tmux.nu

Tmux session management and interrogation tool.

### Available Tools

- `list_sessions` - List all tmux sessions with windows and panes
- `send_command` - Send commands to specific panes
- `capture_pane` - Capture content from panes
- `get_session_info` - Get detailed session information
- `get_pane_process` - Get process information for panes
- `find_pane_by_name` - Find panes by exact name
- `find_pane_by_context` - Find panes by context (directory, command, description)
- `list_panes` - List all panes in a session

### Key Features

- Context-aware pane finding (e.g., "docs", "build")
- Structured table output
- Command logging

## Installation

### Nix
```bash
nix profile install .#mcp-tools
```

Installs to `~/.nix-profile/share/nushell/mcp-tools/tmux/tmux.nu`

### Manual
Copy `tmux.nu` to your nu-mcp tools directory.

## Usage

Configure nu-mcp:

```yaml
nu-mcp-tmux:
  command: "nu-mcp"
  args:
    - "--tools-dir"
    - "~/.nix-profile/share/nushell/mcp-tools/tmux"
```

Example usage via MCP client:
- List all tmux sessions
- Send commands to specific panes by name or context
- Capture pane output for analysis
- Find panes using contextual information like directory names

## Requirements

- tmux installed and in PATH
- nu-mcp server
- Nushell