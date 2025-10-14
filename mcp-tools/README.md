# MCP Tools for Laio

MCP (Model Context Protocol) tools for extending functionality through the `nu-mcp` server.

## tmux.nu

Tmux session management and interrogation tool.

### Available Tools

- `list_sessions` - List all tmux sessions with windows and panes
- `send_and_capture` - **PREFERRED**: Send commands and capture output (interactive)
- `send_command` - Send commands without waiting for output (fire-and-forget)
- `capture_pane` - Capture current visible content (static snapshot)
- `get_session_info` - Get detailed session information
- `get_pane_process` - Get process information for panes
- `find_pane_by_name` - Find panes by exact name
- `find_pane_by_context` - Find panes by context (directory, command, description)
- `list_panes` - List all panes in a session

### Key Features

- **Intelligent command execution**: `send_and_capture` with exponential back-off polling
- **LLM-optimized tool selection**: Clear naming patterns guide AI assistants to the right tool
- **Context-aware pane finding**: Find panes by name, directory, or process (e.g., "docs", "build")
- **Structured table output**: All results formatted for easy reading
- **Command logging**: Full execution tracing for debugging

### Tool Selection Guide

**For interactive commands that need output** (builds, tests, git status):
```
send_and_capture - Automatically handles timing and captures results
```

**For fire-and-forget commands** (starting processes, background tasks):
```
send_command - Returns immediately without waiting
```

**For viewing current pane content** (checking status, reading logs):
```
capture_pane - Static snapshot of what's currently displayed
```

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
- List all tmux sessions and their current state
- **Execute commands and get results**: `send_and_capture` for builds, tests, git operations
- **Start background processes**: `send_command` for long-running tasks
- **Check current status**: `capture_pane` for viewing logs or current output
- **Find panes intelligently**: Search by name, directory, or running process
- **Get detailed information**: Session stats, process info, pane layouts

## Requirements

- tmux installed and in PATH
- nu-mcp server
- Nushell
