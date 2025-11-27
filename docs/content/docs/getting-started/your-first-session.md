+++
title = "Your First Session"
description = "Create a custom session configuration from scratch."
draft = false
weight = 16
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Understanding Sessions

A laio session is defined by a YAML configuration file that describes:
- The session name
- Root working directory
- Windows and their layouts
- Panes within each window
- Commands to run in each pane
- Session lifecycle hooks

## Creating a Configuration

Create a new configuration:

```bash
laio config create myproject
```

This creates `~/.config/laio/myproject.yaml` with a default template.

## Editing the Configuration

Open the configuration in your editor:

```bash
laio config edit myproject
```

Or edit it directly:

```bash
$EDITOR ~/.config/laio/myproject.yaml
```

## Basic Configuration Structure

Here's a simple two-window setup:

```yaml
name: myproject
path: /path/to/myproject

windows:
  - name: editor
    panes:
      - commands:
          - command: nvim

  - name: terminal
    flex_direction: row
    panes:
      - flex: 1
      - flex: 1
```

This creates:
- Window 1: Single pane running `nvim`
- Window 2: Two side-by-side panes (50/50 split)

## Starting Your Session

Start the configured session:

```bash
laio start myproject
```

You'll be automatically attached to the session with your configured layout.

## Validating Configuration

Before starting, validate your YAML:

```bash
laio config validate myproject
```

This checks for syntax errors and schema violations.

## Next Steps

- Learn about [layout options](/docs/configuration/layouts)
- Add [lifecycle hooks](/docs/configuration/lifecycle)
- Explore [YAML reference](/docs/configuration/yaml-reference)
