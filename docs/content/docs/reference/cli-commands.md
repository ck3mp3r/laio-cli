+++
title = "CLI Commands"
description = "Complete reference for all laio CLI commands."
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Global Options

```
--config-dir <PATH>    Config directory (default: ~/.config/laio)
-v, --verbose          Increase logging verbosity (repeat for more)
-q, --quiet            Decrease logging verbosity
-h, --help             Print help
-V, --version          Print version
```

## laio start

Start a new session from a configuration.

### Usage

```bash
laio start [OPTIONS] [NAME]
```

### Arguments

`[NAME]` - Name of the configuration (optional)
- If omitted, shows configuration picker
- If `.laio.yaml` exists, uses it automatically
- Use `-p` to skip local config and show picker

### Options

```
-f, --file <PATH>      Use specific configuration file
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
-p, --show-picker      Show config picker (skip .laio.yaml)
--skip-cmds            Skip startup commands/scripts
--skip-attach          Start session without attaching
--var <KEY=VALUE>      Template variable (repeatable)
```

### Template Variables

Pass variables to your configuration templates using `--var`:

```bash
# Single variable
laio start myconfig --var project_name=webapp

# Multiple variables
laio start myconfig --var name=myapp --var env=dev

# Array variables (repeat same key)
laio start myconfig --var service=api --var service=web --var service=worker
```

Variables can be used in your YAML configuration with Tera syntax:

```yaml
name: {{ project_name }}
path: ~/projects/{{ project_name }}
```

See the [YAML Reference](/docs/configuration/yaml-reference#template-variables) for detailed template variable documentation.

### Examples

```bash
# Start with auto-detection
laio start

# Start specific config
laio start myproject

# Start with variables
laio start template --var project=frontend --var env=dev

# Start with array variables for multiple services
laio start microservices \
  --var services=auth \
  --var services=api \
  --var services=frontend

# Start from custom file
laio start --file /path/to/config.yaml

# Start without startup commands
laio start myproject --skip-cmds

# Start without attaching
laio start myproject --skip-attach

# Force picker (ignore .laio.yaml)
laio start -p
```

## laio stop

Stop a running session.

### Usage

```bash
laio stop [OPTIONS] [NAME]
```

### Arguments

`[NAME]` - Name of the session to stop (optional)
- If omitted, shows session picker

### Options

```
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
--skip-cmds            Skip shutdown commands/scripts
-a, --all              Stop all laio-managed sessions
-o, --others           Stop all sessions except current
```

### Examples

```bash
# Stop specific session
laio stop myproject

# Stop without shutdown commands
laio stop myproject --skip-cmds

# Stop all laio sessions
laio stop --all

# Stop all except current
laio stop --others
```

## laio list

List active and available sessions/configurations.

### Usage

```bash
laio list [OPTIONS]
```

Alias: `laio ls`

### Options

```
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
-j, --json             Output as JSON
```

### Examples

```bash
# List all (table format)
laio list

# List with JSON output
laio list --json

# Using alias
laio ls
```

### Output

Active sessions are marked with `*`.

## laio config

Manage configurations.

### laio config create

Create a new configuration.

#### Usage

```bash
laio config create [OPTIONS] [NAME]
```

#### Arguments

`[NAME]` - Name of the configuration (optional)
- If omitted, creates `.laio.yaml` in current directory

#### Options

```
-c, --copy <NAME>      Copy from existing configuration
```

#### Examples

```bash
# Create new config
laio config create myproject

# Create from template
laio config create newproject --copy template

# Create local .laio.yaml
laio config create
```

### laio config edit

Edit a configuration in `$EDITOR`.

#### Usage

```bash
laio config edit <NAME>
```

#### Arguments

`<NAME>` - Name of the configuration (required)

#### Examples

```bash
laio config edit myproject
```

### laio config link

Link local configuration to global config directory.

#### Usage

```bash
laio config link [OPTIONS] <NAME>
```

#### Arguments

`<NAME>` - Name for the symlink (required)

#### Options

```
-f, --file <PATH>      File to link (default: .laio.yaml)
```

#### Examples

```bash
# Link .laio.yaml
laio config link myproject

# Link custom file
laio config link myproject --file custom-config.yaml
```

### laio config validate

Validate a configuration against the schema.

#### Usage

```bash
laio config validate [OPTIONS] [NAME]
```

#### Arguments

`[NAME]` - Name of the configuration (optional)
- If omitted, validates `.laio.yaml` or prompts

#### Options

```
-f, --file <PATH>      File to validate (default: .laio.yaml)
```

#### Examples

```bash
# Validate named config
laio config validate myproject

# Validate local file
laio config validate --file .laio.yaml

# Validate default local config
laio config validate
```

### laio config delete

Delete a configuration.

#### Usage

```bash
laio config delete [OPTIONS] <NAME>
```

Alias: `laio config rm`

#### Arguments

`<NAME>` - Name of the configuration (required)

#### Options

```
-f, --force            Skip confirmation prompt
```

#### Examples

```bash
# Delete with confirmation
laio config delete myproject

# Force delete
laio config delete myproject --force

# Using alias
laio config rm myproject
```

### laio config list

List all configurations.

#### Usage

```bash
laio config list [OPTIONS]
```

Alias: `laio config ls`

#### Options

```
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
-j, --json             Output as JSON
```

#### Examples

```bash
# List all configs
laio config list

# List with JSON output
laio config list --json

# Using alias
laio config ls
```

## laio session

Manage sessions.

### laio session list

List active sessions.

#### Usage

```bash
laio session list [OPTIONS]
```

Alias: `laio session ls`

#### Options

```
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
-j, --json             Output as JSON
```

#### Examples

```bash
# List active sessions
laio session list

# List with JSON output
laio session list --json
```

### laio session yaml

Export current session to YAML format.

#### Usage

```bash
laio session yaml [OPTIONS]
```

#### Options

```
-m, --muxer <MUXER>    Multiplexer to use (tmux or zellij)
```

#### Examples

```bash
# Print YAML to stdout
laio session yaml

# Save to file
laio session yaml > ~/.config/laio/newsession.yaml

# Save as local config
laio session yaml > .laio.yaml
```

**Note:** Must be run from within a tmux session.

## laio completion

Generate shell completion scripts.

### Usage

```bash
laio completion <SHELL>
```

### Arguments

`<SHELL>` - Shell to generate completions for
- `bash`
- `zsh`
- `fish`
- `nushell`

### Examples

```bash
# Generate bash completions
laio completion bash

# Install bash completions
laio completion bash > ~/.local/share/bash-completion/completions/laio

# Generate zsh completions
laio completion zsh > ~/.zsh/completions/_laio

# Generate fish completions
laio completion fish > ~/.config/fish/completions/laio.fish

# Generate nushell completions (nushell syntax)
laio completion nushell | save ($env.HOME | path join ".config" "nushell" "completions" "laio.nu")
```

## Environment Variables

### LAIO_MUXER

Override the default multiplexer.

```bash
export LAIO_MUXER=tmux   # Use tmux (default)
export LAIO_MUXER=zellij # Use zellij (experimental)
```

Equivalent to using `--muxer` flag on every command.

### LAIO_CONFIG

Used internally by laio to pass configuration to sessions. Do not set manually.

## Tmux Integration

### Session Switching

Laio automatically configures a tmux key binding for quick session switching:

```
prefix M-l
```

This shows a list of all laio-managed sessions for quick switching. The binding is only available within sessions started by laio.

## Known Limitations

### Nested Panes

There are practical limits to nesting depth. Very deep nesting (5+ levels) may cause layout issues or unexpected behavior.

**Recommendations:**
- Keep nesting to 2-3 levels maximum
- Use multiple windows for complex multi-pane setups
- Test configurations before committing to a workflow

### Multiplexer Limitations

**Zellij support is experimental** and may have limitations compared to tmux. Use tmux for production workflows.
