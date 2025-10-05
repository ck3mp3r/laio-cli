# MCP Tools for Laio

This directory contains MCP (Model Context Protocol) tools that extend functionality through the `nu-mcp` server. These tools provide specialized capabilities for managing and interrogating tmux sessions.

## Available Tools

### tmux.nu
Comprehensive tmux session management and interrogation tool that provides:

- **Session Management**: List all tmux sessions with detailed information
- **Command Execution**: Send commands to specific tmux panes
- **Content Capture**: Retrieve output from tmux panes
- **Smart Pane Finding**: Find panes by name or context (directory, command, description)
- **Detailed Information**: Get process information and pane details

#### Key Features

- **Context-aware pane resolution**: Can find panes by contextual information like "docs pane", "build pane", etc.
- **Nested table output**: Provides well-structured, expandable table views of session information
- **Command logging**: All tmux commands are logged for transparency
- **Comprehensive error handling**: Robust error messages and suggestions

#### Available Tools

1. `list_sessions` - List all tmux sessions with windows and panes
2. `send_command` - Send commands to specific panes
3. `capture_pane` - Capture content from panes
4. `get_session_info` - Get detailed session information with nested tables
5. `get_pane_process` - Get process information for specific panes
6. `find_pane_by_name` - Find panes by their exact name/title
7. `find_pane_by_context` - Find panes by context (directory, command, description)
8. `list_panes` - List all panes in a session as an organized table

## Installation

### Using Nix (Recommended)

Install the mcp-tools package:

```bash
nix profile install .#mcp-tools
```

This will install the tools to `~/.nix-profile/share/nushell/mcp-tools/tmux/tmux.nu`.

For building only (without installing):

```bash
nix build .#mcp-tools
```

### Manual Installation

Copy the `.nu` files to your nu-mcp tools directory:

```bash
cp mcp-tools/*.nu /path/to/your/nu-mcp/tools/
```

## Usage with nu-mcp

### Configuration

Configure your MCP client to use nu-mcp with the tools directory:

```yaml
nu-mcp-tmux:
  command: "nu-mcp"
  args:
    - "--tools-dir=~/.nix-profile/share/nushell/mcp-tools/tmux"
```

Or if you installed manually:

```yaml
nu-mcp-tmux:
  command: "nu-mcp"
  args:
    - "--tools-dir=/path/to/your/mcp-tools"
```

### Example Usage

Once configured, you can use these tools through your MCP client:

```javascript
// List all tmux sessions
await client.callTool("list_sessions", {});

// Send a command to a specific pane
await client.callTool("send_command", {
  session: "my-session",
  pane: "docs",  // Can use pane names or context
  command: "ls -la"
});

// Find a pane by context
await client.callTool("find_pane_by_context", {
  session: "my-session", 
  context: "docs"  // Finds panes in docs directory or with "docs" in name
});

// Capture pane content
await client.callTool("capture_pane", {
  session: "my-session",
  pane: "build",
  lines: 50
});
```

## Requirements

- **tmux**: Must be installed and available in PATH
- **nu-mcp**: The Nushell MCP server
- **Nushell**: Compatible with the version used by nu-mcp

## Security Notes

- These tools execute tmux commands with the same privileges as the nu-mcp server
- Be cautious when using in multi-user environments
- The tools can access any tmux session visible to the running user
- Commands sent via `send_command` are executed in the target pane's shell environment

## Development

The tools follow the standard nu-mcp extension pattern:

- `main list-tools` - Returns JSON array of available tools
- `main call-tool <tool_name> <args>` - Executes the specified tool

All tmux commands follow a consistent logging pattern:
1. Build command arguments as a list
2. Log the command with "Executing:" prefix
3. Execute with `run-external`

This pattern ensures transparency and debugging capability.

## Examples

### Finding the "docs" pane and sending a command

```bash
# The tool can intelligently find panes by context
# This will find panes in directories named "docs" or with "docs" in their title
find_pane_by_context "my-session" "docs"

# Then send a command to build documentation
send_command "my-session" "zola build" null "docs"
```

### Getting a comprehensive session overview

```bash
# Get detailed information with nested tables
get_session_info "my-session"

# Or list all panes in an organized table
list_panes "my-session"
```

## Contributing

When adding new tools:

1. Follow the existing pattern in `tmux.nu`
2. Use the `exec_tmux_command` helper for consistent logging
3. Provide comprehensive error handling
4. Include detailed tool descriptions and input schemas
5. Test with various tmux configurations

## Disclaimer

**USE AT YOUR OWN RISK**: These tools directly interact with tmux sessions and can execute commands in terminal environments. Users are responsible for understanding the security implications and testing thoroughly before production use.