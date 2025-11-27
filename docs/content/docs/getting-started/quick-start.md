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

This creates a default configuration with:
- An editor window with your `$EDITOR`
- A terminal window with two vertically split panes

## View Available Sessions

List all sessions and configurations:

```bash
laio list
```

Active sessions are marked with `*`.

## Stop a Session

Stop the session when you're done:

```bash
laio stop myproject
```

## What's Next?

- Learn more about [creating configurations](/docs/getting-started/your-first-session)
- Explore [configuration options](/docs/configuration/yaml-reference)
- Understand [workflow patterns](/docs/workflow/managing-configs)
