+++
title = "Advanced"
description = "Advanced laio."
draft = false
weight = 25
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

## Project Based Or Local Configurations

Not all laio configurations need to be stored in `~/.config/laio`, they can also reside in a project directory.
When you run `laio start` within a directory, laio will look for a file named `.laio.yaml` and also search up until it reaches the users home directory.

### Creating Local Configurations

Create a `.laio.yaml` in your project directory:
```bash
cd myproject
laio config create         # creates .laio.yaml in current directory
laio config create --copy existing-config  # copy from existing config
```

### Linking Local Configurations

The only caveat using this approach is that 
```bash
laio ls
```
will not list project based laio configurations. 
If you want a project based laio configuration to be visible to laio during normal operation, you can link the local `.laio.yaml` using:
```bash
laio config link <name>
```
You can also specify a custom file to link:
```bash
laio config link <name> --file custom-config.yaml
```

This creates a symbolic link from `.laio.yaml` into `~/.config/laio/<name>.yaml`, after which:
```bash
laio start <name>
```
or 
```bash
laio ls 
```
will also pick up on the configuration.

## Saving Existing Tmux Sessions

Alternatively to creating new configurations manually or via 
```bash
laio config create <name> 
```
you can also use laio to create a config file from within the tmux session you are already in.
```bash
laio session yaml > ~/.config/laio/<name>.yaml
```
This will serialize the current tmux session into the right format and save it to the file specified.

You can also preview the YAML output:
```bash
laio session yaml
```

This feature is useful for:
- Capturing complex layouts you've built interactively
- Creating templates from working sessions
- Documenting session configurations
- Sharing team development environments

### Important Note on Exported Commands

When exporting sessions, laio captures the **actual running commands** as reported by tmux, not the wrapper scripts, aliases, or shell functions you may have used to launch them. For example:

- Shell aliases (e.g., `vim` → `nvim`) will show the underlying command
- Wrapper scripts will show the final executed command
- Shell functions won't be captured, only their executed processes

**You may need to manually edit the exported YAML** to replace process commands with the appropriate startup commands for your workflow. This is especially important for:
- Development servers that use wrapper scripts
- Applications launched via shell functions
- Commands invoked through aliases

## Managing Configurations

Edit a configuration in your `$EDITOR`:
```bash
laio config edit <name>
```

Validate a configuration against the schema:
```bash
laio config validate <name>           # validate named config
laio config validate --file .laio.yaml  # validate local file
```

Delete a configuration:
```bash
laio config delete <name>        # prompts for confirmation
laio config delete <name> --force  # skip confirmation
```

Create a new configuration:
```bash
laio config create <name>              # create from template
laio config create <name> --copy src   # copy from existing config
laio config create                     # creates .laio.yaml locally
```

## Focus and Zoom Features

You can control which pane receives initial focus and whether panes start zoomed:

```yaml
windows:
  - name: dev
    panes:
      - flex: 1
        commands:
          - command: npm run build
      - flex: 2
        focus: true     # cursor starts here
        zoom: true      # pane starts in zoomed state
        commands:
          - command: npm run dev
```

**Focus behavior:**
- Only one pane should have `focus: true`
- If multiple panes have focus, the last one wins
- If no pane has focus, cursor lands in first pane

**Zoom behavior:**
- Zoomed panes hide all other panes in the window
- Use tmux `prefix + z` to toggle zoom manually
- Useful for focusing on logs or primary work pane

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
