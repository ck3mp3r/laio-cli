+++
title = "Quick Start"
description = "Get up and running with laio in minutes."
draft = false
weight = 15
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Create Your First Session

The fastest way to get started with laio is to create a configuration and start it:

```bash
# Create a new configuration
laio config create myproject

# Start the session
laio start myproject
```

> laio targets **tmux** by default. Experimental `zellij`, `herdr`, and `rex` backends are available via `--muxer zellij|herdr|rex` or `LAIO_MUXER` on any command.
>
> rex runs one `laio: <name>` session per config. Attach needs Remote Control enabled in Rex.app under Settings > Rex Server; when the attach fails, the start still succeeds and rex's error is logged as a warning. `env` and pane `style` are ignored, and `laio session yaml` loses pane commands and ignores floating layers. Session records live in `LAIO_STATE_DIR` when set, otherwise `$XDG_STATE_HOME/laio/rex`, otherwise `~/.local/state/laio/rex`.

This creates a default configuration with:
- An editor window that runs `{% raw %}{{ editor | default(value="nvim") }}{% endraw %}` from the template (override with `--var editor=<cmd>`)
- A terminal window with three panes: one pane beside two stacked panes in a nested column split (1:5 ratio)

## View Available Sessions

List all sessions and configurations:

```bash
laio list
```

Each row shows a status icon:

- `●` - attached (running, you are in it)
- `○` - active (running, not attached)
- `·` - inactive (configuration only)

## Stop a Session

Stop the session when you're done:

```bash
laio stop myproject
```

## What's Next?

- Learn more about [creating configurations](/docs/getting-started/your-first-session)
- Explore [configuration options](/docs/configuration/yaml-reference)
- Understand [workflow patterns](/docs/workflow/managing-configs)
