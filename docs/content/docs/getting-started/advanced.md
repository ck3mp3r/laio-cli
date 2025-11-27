+++
title = "Advanced Features"
description = "Advanced laio features and workflows."
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Switching Between Laio Sessions

You can easily switch between laio sessions within tmux using the tmux key binding 
```tmux
prefix M-l
```
Laio automatically configures the key binding within tmux sessions it starts itself.

## Project-Based Configurations

Laio supports project-local configurations stored as `.laio.yaml` in your project directory.

Quick example:
```bash
cd myproject
laio config create         # creates .laio.yaml
laio start                 # auto-detects and uses .laio.yaml
```

Link local configs to make them globally visible:
```bash
laio config link myproject
```

For complete documentation, see [Local Configs](/docs/workflow/local-configs).

## Exporting Existing Sessions

Capture your current tmux session as a laio configuration:
```bash
laio session yaml > ~/.config/laio/<name>.yaml
```

**Important:** Exported commands show actual running processes, not the wrappers/aliases you used. Manual editing is typically required.

For detailed export workflow and limitations, see [Session Export](/docs/workflow/session-export).

## Configuration Management

Common configuration operations:

```bash
laio config create <name>              # create from template
laio config create <name> --copy src   # copy from existing config
laio config edit <name>                # edit in $EDITOR
laio config validate <name>            # validate against schema
laio config delete <name> --force      # delete configuration
```

For complete reference, see [Managing Configs](/docs/workflow/managing-configs).

## Focus and Zoom

Control cursor placement and pane zoom on session start:

```yaml
panes:
  - flex: 1
  - flex: 2
    focus: true     # cursor starts here
    zoom: true      # pane starts zoomed
```

See [Layouts](/docs/configuration/layouts) for details on focus, zoom, and layout options.

## Multiplexer Support

Laio primarily supports tmux. Experimental Zellij support is available via the `--muxer` flag or `LAIO_MUXER` environment variable.

```bash
# Use Zellij (experimental)
laio start myproject --muxer zellij

# Or set environment variable
export LAIO_MUXER=zellij
laio start myproject
```

**Note:** Zellij support is experimental and may have limitations.
