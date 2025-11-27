+++
title = "Session Export"
description = "Export existing tmux sessions to YAML configuration."
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Overview

Laio can serialize existing tmux sessions into YAML configuration format. This is useful for:

- Capturing complex layouts built interactively
- Creating configurations from working sessions
- Documenting current session structure
- Sharing team development environments
- Creating templates from proven setups

## Basic Export

From within a tmux session, export to YAML:

```bash
laio session yaml
```

This outputs the current session structure to stdout.

## Saving to File

Save the exported YAML to a configuration file:

```bash
laio session yaml > ~/.config/laio/mysession.yaml
```

Or create a local configuration:

```bash
laio session yaml > .laio.yaml
```

## What Gets Exported

The export captures:

- **Session name** - Current tmux session name
- **Window structure** - All windows and their names
- **Pane layout** - Splits, sizes, and arrangement
- **Working directories** - Current path for each pane
- **Running commands** - Active processes in panes (see limitations below)

## What Doesn't Get Exported

The following are **not** captured:

- Session-level `path` (set to `.` by default)
- Environment variables (`env`)
- Lifecycle hooks (`startup`, `shutdown`, `startup_script`, `shutdown_script`)
- Pane styles (`style`)
- Focus and zoom states (`focus`, `zoom`)

You'll need to add these manually after export.

## Command Limitations

**Important:** Exported commands reflect **actual running processes**, not the wrapper scripts, aliases, or shell functions used to launch them.

### Examples of Command Translation

| You Ran | Exported Command |
|---------|------------------|
| `vim` (alias to nvim) | `nvim` (actual binary) |
| `dev` (shell function) | `node server.js` (underlying process) |
| `npm run dev` | `node --loader tsx src/index.ts` (actual command) |
| Shell with no command | Empty (no `commands` field) |

### Manual Editing Required

After export, you'll likely need to edit commands:

```yaml
# Exported
panes:
  - commands:
      - command: /usr/local/bin/node
        args: [--loader, tsx, src/index.ts]

# Better for re-use
panes:
  - commands:
      - command: npm
        args: [run, dev]
```

## Typical Workflow

### 1. Build Your Layout Interactively

Create panes and run commands manually in tmux:

```bash
tmux new -s experiment
# Create windows, split panes, run commands
# Adjust layout until satisfied
```

### 2. Export the Session

```bash
laio session yaml > ~/.config/laio/experiment.yaml
```

### 3. Edit the Configuration

```bash
laio config edit experiment
```

Add/fix:
- Session `path`
- Lifecycle hooks if needed
- Correct command wrappers
- Environment variables
- Focus and zoom preferences

### 4. Test the Configuration

```bash
tmux kill-session -t experiment  # Stop the original
laio start experiment             # Test the config
```

### 5. Iterate

Refine the configuration based on testing.

## Example Export

### Interactive Session

Create a session manually:

```bash
tmux new -s myapp -c ~/projects/myapp
tmux split-window -h -c ~/projects/myapp
tmux split-window -v -c ~/projects/myapp
tmux select-pane -t 0
tmux split-window -v -c ~/projects/myapp
# Run various commands in panes
```

### Exported YAML

```bash
laio session yaml
```

Produces something like:

```yaml
name: myapp
path: .

windows:
  - name: myapp
    flex_direction: row
    panes:
      - flex: 1
        flex_direction: column
        panes:
          - flex: 1
            path: /Users/username/projects/myapp
          - flex: 1
            path: /Users/username/projects/myapp
            commands:
              - command: /usr/local/bin/node
                args: [server.js]
      - flex: 1
        flex_direction: column
        panes:
          - flex: 1
            path: /Users/username/projects/myapp
            commands:
              - command: /usr/bin/tail
                args: [-f, logs/app.log]
          - flex: 1
            path: /Users/username/projects/myapp
```

### After Manual Cleanup

```yaml
name: myapp
path: ~/projects/myapp  # Cleaned up path

startup:
  - command: docker-compose
    args: [up, -d]

windows:
  - name: dev
    flex_direction: row
    panes:
      - flex: 1
        flex_direction: column
        panes:
          - flex: 1
            focus: true  # Added
          - flex: 1
            commands:
              - command: npm  # Fixed wrapper
                args: [run, dev]
      - flex: 1
        flex_direction: column
        panes:
          - flex: 1
            zoom: true  # Added
            commands:
              - command: npm  # Fixed
                args: [run, logs]
          - flex: 1
```

## Best Practices

1. **Export as a starting point** - Not as final configuration
2. **Clean up paths** - Replace absolute paths with relative or `~`
3. **Fix commands** - Replace process commands with proper wrappers
4. **Add lifecycle hooks** - Include startup/shutdown as needed
5. **Add metadata** - Set focus, zoom, styles, env vars
6. **Test thoroughly** - Exported configs may not work perfectly first try
7. **Document** - Add comments for complex setups

## Common Use Cases

### Capture Complex Layout

Built a perfect 6-pane layout interactively? Export it:

```bash
laio session yaml > ~/.config/laio/dashboard.yaml
```

### Team Onboarding

Show new team members the ideal dev environment:

```bash
# Senior developer with working setup
laio session yaml > .laio.yaml
git add .laio.yaml
git commit -m "Add team development session"
```

### Multiple Environments

Create different configs for different contexts:

```bash
# Development layout
laio session yaml > ~/.config/laio/dev.yaml

# Switch to debugging layout
tmux kill-session -t dev
# ... set up debugging session ...
laio session yaml > ~/.config/laio/debug.yaml

# Switch to presentation layout
# ... set up clean layout ...
laio session yaml > ~/.config/laio/present.yaml
```

Now switch between them easily:

```bash
laio start dev
laio start debug
laio start present
```

## Troubleshooting

### Empty Commands

If panes show no commands:
- The pane was running a shell with no active process
- Add commands manually for what should run

### Wrong Paths

Exported paths are absolute. Replace with:
- Relative paths (e.g., `./src`)
- Home-relative (e.g., `~/projects/app`)
- Session root (`.`)

### Binary Paths

Commands show full binary paths (e.g., `/usr/local/bin/node`):
- Replace with simple command names (e.g., `node`)
- Use wrapper scripts (e.g., `npm run dev`)

### Layout Doesn't Match

The flex sizing is approximate:
- Adjust `flex` values to get desired proportions
- Test and iterate
