+++
title = "Basics"
description = "Essential laio commands and usage."
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Command Overview

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

See [Managing Configs](/docs/workflow/managing-configs) for more details on creating and editing configurations.

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

windows:
  - name: editor
    panes:
      - commands:
          - command: $EDITOR

  - name: terminal
    flex_direction: row  # horizontal splits
    panes:
      - flex: 1
      - flex: 1
```

For comprehensive configuration documentation, see:
- [YAML Reference](/docs/configuration/yaml-reference) - All fields and options
- [Layouts](/docs/configuration/layouts) - Flexbox layouts, focus, zoom
- [Lifecycle Hooks](/docs/configuration/lifecycle) - Startup/shutdown scripts

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

See [Managing Configs](/docs/workflow/managing-configs) for more configuration management options.

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

This is useful for capturing layouts you've built interactively.

**Note:** The exported commands reflect the actual running processes, not wrapper scripts or shell functions. You may need to manually edit commands in the exported YAML.

See [Session Export](/docs/workflow/session-export) for detailed workflow and limitations.

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

