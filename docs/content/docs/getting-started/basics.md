+++
title = "Basics"
description = "Basic laio."
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Using laio

Once you have `laio` on your path simply running it will output the options available:
```
A simple flexbox-like layout manager for tmux.

Usage: laio [OPTIONS] <COMMAND>

Commands:
  start       Start new session
  stop        Stop session
  list        List active (*) and available sessions
  config      Manage Configurations
  session     Manage Sessions
  completion  Display the completion file for a given shell
  help        Print this message or the help of the given subcommand(s)

Options:
      --config-dir <CONFIG_DIR>  [default: ~/.config/laio]
  -v, --verbose...               Increase logging verbosity
  -q, --quiet...                 Decrease logging verbosity
  -h, --help                     Print help
  -V, --version                  Print version

```

## Creating a Configuration

Using laio requires a configuration that describes the kind of tmux session you want. Config files are usually stored in `~/.config/laio`.
You can also have config files inside project directories named `.laio.yaml`.

To create a new configuration run
```bash
laio config create <name-of-config>
```
This will create a new config with the same session name.
The config is a default 2 window session with the first window being dedicated for `$EDITOR` and the second window consisting of two vertically split panes.

## Starting a Session

To start a session from an existing config run
```bash
laio start <name-of-config>
```
Alternatively, if you omit the config name, you will be presented with a list of known configurations, unless there is a `.laio.yaml` present.

*Note: if the local config cannot be found in the current directory, it will search up the path until reaching the users home directory.*

If you want to skip the local laio config and use the picker to select then run:
```bash
laio start -p
```

You can also specify a custom config file:
```bash
laio start --file /path/to/custom-config.yaml
```

Additional start options:
```bash
laio start myproject --skip-cmds     # Skip startup commands/scripts
laio start myproject --skip-attach   # Start session without attaching
```

## Configuration YAML

A simple yaml configuration looks as follows:
```yaml
---
name: myproject

path: /path/to/myproject

# Session lifecycle - commands run when session starts
startup:
  - command: gh
    args:
      - auth
      - login

# Startup script - runs after startup commands
startup_script: |
  #!/usr/bin/env bash
  echo "Hello from the startup script"

# Shutdown lifecycle - commands run when session stops
shutdown:
  - command: echo
    args:
      - "Bye bye!"

# Shutdown script - runs after shutdown commands
shutdown_script: |
  #!/usr/bin/env bash
  echo "Bye bye from the shutdown script"

shell: /bin/zsh # optional shell for the given session to use

env: # optional environment variables to pass to the session
  FOO: bar
  BAZ: foo

windows:
  - name: code
    panes:
      - name: Editor  # optional pane name
        commands: # starting up system editor in this pane
          - command: $EDITOR

  - name: local
    flex_direction: row # splits are vertical, panes are side by side
    panes:
      - flex: 1 # what proportion of the window to occupy in relation to the other splits
        flex_direction: column # splits are horizontal, panes are on top of each other
        panes:
          - flex: 1
            path: ./foo # path relative to the root path declared above
            style: bg=darkred,fg=default # specify pane styles as per tmux options
            commands:
              - command: colima
                args:
                  - start
                  - --kubernetes
                  - --kubernetes-version
                  - "v1.25.11+k3s1"
                  - --cpu 6
                  - --memory 24
            # Pane-level script - runs after pane commands
            script: |
              #!/usr/bin/env bash
              echo "You can also have custom scripts embedded on a pane level"

          - flex: 6
            focus: true  # this pane will have initial focus
            zoom: false  # optionally start in zoomed state (default: false)
      - flex: 1
```

### Configuration Fields Reference

**Session-level:**
- `name` (required): Session name
- `path` (required): Root directory for the session
- `startup`: List of commands to run when session starts
- `startup_script`: Script to run after startup commands
- `shutdown`: List of commands to run when session stops
- `shutdown_script`: Script to run after shutdown commands
- `shell`: Shell to use (default: system shell)
- `env`: Environment variables as key-value pairs
- `windows` (required): List of window definitions

**Window-level:**
- `name` (required): Window name
- `path`: Working directory (overrides session path)
- `flex_direction`: Layout direction - `row` (horizontal) or `column` (vertical)
- `panes` (required): List of pane definitions

**Pane-level:**
- `name`: Optional pane identifier
- `flex`: Proportional size (e.g., `flex: 2` is twice the size of `flex: 1`)
- `path`: Working directory (overrides window/session path)
- `focus`: Set to `true` to start with cursor in this pane
- `zoom`: Set to `true` to start pane in zoomed state
- `style`: tmux style options (e.g., `bg=blue,fg=white`)
- `flex_direction`: For nested panes - `row` or `column`
- `panes`: Nested panes (supports multiple levels)
- `commands`: List of commands to execute in sequence
- `script`: Inline script to run after commands

## Listing Sessions and Configurations

List active (*) and available sessions:
```bash
laio list
```

List all configurations:
```bash
laio config list
```

Both commands support `--json` or `-j` flag for JSON output.

## Stopping Sessions

Stop a specific session:
```bash
laio stop <name>
```

Stop all laio-managed sessions:
```bash
laio stop --all
```

Stop other sessions (all except current):
```bash
laio stop --others
```

Skip shutdown commands/scripts when stopping:
```bash
laio stop myproject --skip-cmds
```

## Exporting Sessions

You can export an existing tmux session to YAML format:
```bash
# From within a tmux session
laio session yaml

# Save to a file
laio session yaml > ~/.config/laio/mysession.yaml
```

This is useful for:
- Capturing your current working session layout
- Creating new configurations from existing setups
- Sharing session configurations

**Note:** The exported commands reflect the actual running processes, not wrapper scripts or shell functions. You may need to manually edit commands in the exported YAML if your workflow uses aliases, shell functions, or wrapper scripts (e.g., `nvim` might appear as the actual binary path rather than the alias you used).

## Shell Completion

Generate shell completion for your shell:
```bash
laio completion bash
laio completion zsh
laio completion fish
laio completion nushell
```

Install completions (example for bash):
```bash
laio completion bash > ~/.local/share/bash-completion/completions/laio
```

## Known Limitations

Currently there is a known limitation to the number of nested panes allowed.
Play around with the configurations to see what works best for you.

