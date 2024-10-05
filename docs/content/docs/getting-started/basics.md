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

*Note: if the config cannot be found in the current directory, it will search up the path until reaching the users home directory.*

## Configuration YAML

A simple yaml configuration looks as follows:
```yaml
---
name: myproject

path: /path/to/myproject
startup: # a list of startup commands to run
  - gh auth login

shutdown: # a list of shutdown commands to run
  - echo "Bye bye!"

windows:
  - name: code
    panes:
      - commands: # starting up system editor in this pane
          - $EDITOR

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
              - colima start --kubernetes --kubernetes-version "v1.25.11+k3s1" --cpu 6 --memory 24
          - flex: 6
      - flex: 1
```

## Completion

To generate the right shell completion for your shell run 
```bash
laio completion <your-shell>
```

## Known Limitations

Currently there is a known limitation to the number of nested panes allowed. 
Play around with the configurations to see what works best for you.

