+++
title = "Local Configs"
description = "Using project-local .laio.yaml configurations."
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Overview

Local configurations allow you to keep session definitions in your project directory rather than the global `~/.config/laio/` location.

Benefits:
- Version controlled with your project
- Shared with team members
- Project-specific without polluting global config
- Automatic detection when running `laio start`

## Creating Local Configs

Create `.laio.yaml` in your project directory:

```bash
cd ~/projects/myapp
laio config create
```

This creates `.laio.yaml` in the current directory.

Or copy from an existing config:

```bash
laio config create --copy template
```

## Using Local Configs

When you run `laio start` without a name argument:

```bash
laio start
```

Laio searches for `.laio.yaml`:
1. Current directory
2. Parent directories up to `$HOME`
3. If not found, shows configuration picker

Example directory structure:

```
~/projects/
└── myapp/
    ├── .laio.yaml      ← Found here
    ├── backend/
    │   └── src/
    └── frontend/
        └── src/
```

Running `laio start` from any subdirectory (e.g., `backend/src/`) will find and use the root `.laio.yaml`.

## Skipping Local Config

Force the configuration picker:

```bash
laio start -p
```

Or use a specific file:

```bash
laio start --file /path/to/other-config.yaml
```

## Linking Local Configs

Make a local config visible globally:

```bash
cd ~/projects/myapp
laio config link myapp
```

This creates a symlink:
```
~/.config/laio/myapp.yaml → ~/projects/myapp/.laio.yaml
```

Now you can:

```bash
laio start myapp          # Works from anywhere
laio list                 # Shows myapp in list
laio config edit myapp    # Opens the linked file
```

### Custom Link File

Link a different file:

```bash
laio config link myapp --file custom-config.yaml
```

## Validation

Validate local configuration:

```bash
laio config validate --file .laio.yaml
```

Or if linked:

```bash
laio config validate myapp
```

## Version Control

Add `.laio.yaml` to your repository:

```bash
git add .laio.yaml
git commit -m "Add laio session configuration"
```

Consider adding a README note:

```markdown
## Development Setup

This project uses [laio](https://laio.sh) for session management.

1. Install laio
2. Run `laio start` in the project root
```

### Gitignore Patterns

You might want to ignore environment-specific customizations:

```gitignore
# Keep the main config
!.laio.yaml

# Ignore local overrides (if you create them)
.laio.local.yaml
```

## Team Workflow

### Setup

1. Project maintainer creates `.laio.yaml`:
   ```bash
   laio config create
   git add .laio.yaml
   git commit -m "Add laio configuration"
   ```

2. Team members clone and start:
   ```bash
   git clone <repo>
   cd <project>
   laio start
   ```

### Customization

Team members can:

1. Use the shared config as-is:
   ```bash
   laio start
   ```

2. Link it for global access:
   ```bash
   laio config link projectname
   ```

3. Create personal overrides (not committed):
   ```bash
   cp .laio.yaml .laio.local.yaml
   # Edit .laio.local.yaml for personal preferences
   laio start --file .laio.local.yaml
   ```

## Limitations

Local configurations are **not** listed by default:

```bash
laio list  # Won't show local configs unless linked
```

To make them visible, link them:

```bash
laio config link <name>
```

## Common Patterns

### Multi-Project Workspace

If you have multiple projects with `.laio.yaml`:

```
~/projects/
├── api/
│   └── .laio.yaml
├── web/
│   └── .laio.yaml
└── mobile/
    └── .laio.yaml
```

Link them all:

```bash
cd ~/projects/api && laio config link api
cd ~/projects/web && laio config link web
cd ~/projects/mobile && laio config link mobile
```

Now you can start any project from anywhere:

```bash
laio start api
laio start web
laio start mobile
```

### Project Templates

Create a `.laio.yaml` template for new projects:

```yaml
# .laio.yaml template
name: CHANGEME
path: .

windows:
  - name: editor
    panes:
      - commands:
          - command: $EDITOR
  
  - name: dev
    panes:
      - flex: 1
      - flex: 1
```

Copy it to new projects and customize:

```bash
cp ~/templates/.laio.yaml ~/projects/newproject/
cd ~/projects/newproject
# Edit .laio.yaml, update name
laio start
```

### Monorepo Setup

For monorepos, create `.laio.yaml` at the root:

```yaml
name: monorepo
path: .

windows:
  - name: api
    panes:
      - path: ./packages/api
        commands:
          - command: npm
            args: [run, dev]
  
  - name: web
    panes:
      - path: ./packages/web
        commands:
          - command: npm
            args: [run, dev]
  
  - name: mobile
    panes:
      - path: ./packages/mobile
        commands:
          - command: npm
            args: [run, start]
```

Each window works in its respective package directory.
