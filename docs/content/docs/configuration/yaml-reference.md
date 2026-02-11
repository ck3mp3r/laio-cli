+++
title = "YAML Reference"
description = "Complete reference for laio YAML configuration."
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Session-Level Fields

### Required Fields

**`name`** (string)
- Session name used by tmux
- Must be unique across all sessions
- Example: `"myproject"`

**`path`** (string)
- Root directory for the session
- Can be absolute or use `~` for home directory
- Example: `"/path/to/project"` or `"~/projects/myapp"`

**`windows`** (array)
- List of window definitions
- At least one window required
- See [Window-Level Fields](#window-level-fields)

### Optional Fields

**`shell`** (string)
- Shell to use for the session
- Overrides system default shell
- Example: `"/bin/zsh"`, `"/bin/bash"`

**`env`** (object)
- Environment variables as key-value pairs
- Available to all commands in the session
- Example:
  ```yaml
  env:
    NODE_ENV: development
    DEBUG: "app:*"
  ```

**`startup`** (array of commands)
- Commands to run when session starts
- Executed before panes are created
- See [Command Structure](#command-structure)

**`startup_script`** (string)
- Inline script to run after startup commands
- Must include shebang (e.g., `#!/usr/bin/env bash`)
- Example:
  ```yaml
  startup_script: |
    #!/usr/bin/env bash
    echo "Session starting..."
  ```

**`shutdown`** (array of commands)
- Commands to run when session stops
- Executed after panes are destroyed
- See [Command Structure](#command-structure)

**`shutdown_script`** (string)
- Inline script to run after shutdown commands
- Must include shebang
- Example:
  ```yaml
  shutdown_script: |
    #!/usr/bin/env bash
    echo "Cleaning up..."
  ```

## Template Variables

laio supports template variables using the [Tera](https://keats.github.io/tera/) template engine, allowing you to create flexible, reusable configurations.

### Basic Usage

Use `{{ variable }}` syntax in your YAML configuration and pass values via the `--var` flag:

```yaml
name: {{ project_name }}
path: ~/projects/{{ project_name }}

windows:
  - name: editor
    panes:
      - commands:
          - command: {{ editor | default(value="nvim") }}
```

Start the session with variables:

```bash
laio start myconfig --var project_name=webapp --var editor=vim
```

### Default Values

Variables can have default values using the `default` filter:

```yaml
name: {{ name | default(value="my-session") }}
path: {{ path | default(value="~") }}
shell: {{ shell | default(value="/bin/zsh") }}
```

If a variable isn't provided, the default value is used:

```bash
# Uses defaults for all variables
laio start myconfig

# Overrides only the name
laio start myconfig --var name=custom-session
```

### Array Variables

Repeat the same `--var` key multiple times to create arrays for use in loops:

```yaml
name: {{ name | default(value="multi-env") }}
path: ~/projects

windows:
{% for env in environments %}
  - name: {{ env }}
    panes:
      - path: ./{{ env }}
        commands:
          - command: npm
            args: [run, dev]
        env:
          NODE_ENV: {{ env }}
{% endfor %}
```

Pass array values:

```bash
laio start --var name=myapp --var environments=dev --var environments=staging --var environments=prod
```

This creates three windows: one for dev, one for staging, and one for prod.

### Common Use Cases

#### Project-Specific Configurations

Create a generic template for different projects:

```yaml
name: {{ project }}
path: ~/projects/{{ project }}

env:
  PROJECT_NAME: {{ project }}
  ENV: {{ env | default(value="development") }}

windows:
  - name: editor
    panes:
      - commands:
          - command: $EDITOR
            args: [{{ main_file | default(value="README.md") }}]
```

Use it for different projects:

```bash
laio start template --var project=frontend --var main_file=src/App.tsx
laio start template --var project=backend --var main_file=main.rs --var env=staging
```

#### Git Worktrees

Manage multiple git worktrees easily:

```yaml
name: {{ repo }}-{{ branch }}
path: ~/worktrees/{{ repo }}/{{ branch }}

windows:
  - name: code
    panes:
      - commands:
          - command: git
            args: [status]
          - command: $EDITOR
```

Create worktree sessions:

```bash
laio start worktree --var repo=myproject --var branch=feature-auth
laio start worktree --var repo=myproject --var branch=bugfix-login
```

#### Multi-Service Development

Start multiple microservices dynamically:

```yaml
name: {{ stack | default(value="microservices") }}
path: ~/projects/{{ stack }}

windows:
{% for service in services %}
  - name: {{ service }}
    flex_direction: column
    panes:
      - flex: 3
        path: ./{{ service }}
        commands:
          - command: npm
            args: [run, dev]
      - flex: 1
        path: ./{{ service }}
        commands:
          - command: npm
            args: [run, test:watch]
{% endfor %}
```

Launch your stack:

```bash
laio start microservices \
  --var services=auth \
  --var services=api \
  --var services=frontend \
  --var services=worker
```

### Tera Template Syntax

laio uses Tera's template syntax. Key features:

**Variables:**
- `{{ variable }}` - Insert variable value
- `{{ variable | default(value="fallback") }}` - Default value if undefined

**Loops:**
```yaml
{% for item in items %}
  - name: {{ item }}
{% endfor %}
```

**Conditionals:**
```yaml
{% if debug %}
  env:
    DEBUG: "true"
{% endif %}
```

**Filters:**
- `{{ name | upper }}` - Convert to uppercase
- `{{ name | lower }}` - Convert to lowercase
- `{{ path | replace(from="~", to="/home/user") }}` - Replace text

See the [Tera documentation](https://keats.github.io/tera/docs/) for complete syntax reference.

### Troubleshooting

**Error: "Template rendering failed: Variable `x` not found"**
- Variable is used without a default value
- Solution: Add a default value or pass the variable via `--var`

**Error: "Invalid variable format: 'key'"**
- Missing `=` in `--var` argument
- Solution: Use format `--var key=value`

**Templates not rendering:**
- Check that you're using `{{ }}` syntax, not `{ }` (old format)
- Ensure variable names match exactly (case-sensitive)

**Array variables not working:**
- Ensure you're repeating the same key: `--var items=a --var items=b`
- In templates, use `{% for item in items %}` not `{{ items }}`

## Window-Level Fields

### Required Fields

**`name`** (string)
- Window name displayed in tmux status bar
- Example: `"editor"`, `"logs"`

**`panes`** (array)
- List of pane definitions
- At least one pane required
- See [Pane-Level Fields](#pane-level-fields)

### Optional Fields

**`path`** (string)
- Working directory for all panes in this window
- Overrides session-level `path`
- Relative paths are relative to session path
- Example: `"./src"`, `"/tmp"`

**`flex_direction`** (string: `"row"` or `"column"`)
- Layout direction for panes
- `"row"`: vertical split (panes side-by-side, left/right)
- `"column"`: horizontal split (panes stacked, top/bottom)
- Default: `"row"`

## Pane-Level Fields

### Optional Fields

**`name`** (string)
- Optional identifier for the pane
- Not displayed in tmux, used for documentation
- Example: `"main-terminal"`

**`flex`** (number)
- Proportional size relative to sibling panes
- Higher values = larger panes
- Example: `flex: 2` is twice the size of `flex: 1`
- Required if pane has siblings

**`path`** (string)
- Working directory for this pane
- Overrides window and session paths
- Example: `"./logs"`, `"~/downloads"`

**`focus`** (boolean)
- Set to `true` to place cursor in this pane on start
- Only one pane should have `focus: true`
- Default: `false`

**`zoom`** (boolean)
- Set to `true` to start pane in zoomed state
- Hides all other panes in the window
- Default: `false`

**`style`** (string)
- tmux pane styling options
- Format: comma-separated key=value pairs
- Example: `"bg=blue,fg=white"`, `"bg=darkred,fg=default"`
- See `man tmux` for available style options

**`flex_direction`** (string: `"row"` or `"column"`)
- Layout direction for nested panes
- Only applies if `panes` is defined
- Default: `"row"`

**`panes`** (array)
- Nested pane definitions
- Supports multiple levels of nesting
- See [Pane-Level Fields](#pane-level-fields)

**`commands`** (array of commands)
- Commands to execute in sequence in this pane
- See [Command Structure](#command-structure)

**`script`** (string)
- Inline script to run after commands
- Must include shebang
- Runs after all commands complete
- Example:
  ```yaml
  script: |
    #!/usr/bin/env bash
    tail -f app.log
  ```

## Command Structure

Commands can be defined in two formats:

### Simple Format

For commands without arguments:

```yaml
commands:
  - command: nvim
```

### Full Format

For commands with arguments:

```yaml
commands:
  - command: npm
    args:
      - run
      - dev
      - --port
      - "3000"
```

### Field Reference

**`command`** (string, required)
- Command to execute
- Can include environment variables (e.g., `$EDITOR`)

**`args`** (array of strings, optional)
- Command arguments
- Each argument as a separate array item

## Complete Example

```yaml
name: fullstack-app
path: ~/projects/my-app

shell: /bin/zsh

env:
  NODE_ENV: development
  API_URL: http://localhost:8000

startup:
  - command: docker-compose
    args: [up, -d, postgres]

startup_script: |
  #!/usr/bin/env bash
  sleep 2
  npm run db:migrate

shutdown:
  - command: docker-compose
    args: [down]

windows:
  - name: editor
    panes:
      - name: nvim
        focus: true
        commands:
          - command: $EDITOR

  - name: dev
    flex_direction: row  # vertical split: side-by-side
    panes:
      - flex: 2
        flex_direction: column  # horizontal split: stacked
        panes:
          - flex: 3
            path: ./backend
            commands:
              - command: npm
                args: [run, dev]
          - flex: 1
            commands:
              - command: npm
                args: [run, test:watch]

      - flex: 1
        path: ./frontend
        commands:
          - command: npm
            args: [run, dev]

  - name: logs
    panes:
      - zoom: true
        style: bg=black,fg=green
        script: |
          #!/usr/bin/env bash
          docker-compose logs -f
```

## Schema File

The JSON schema is available at `src/common/config/schema.json` in the repository for editor integration and validation.
