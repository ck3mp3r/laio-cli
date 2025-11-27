+++
title = "Managing Configs"
description = "Create, edit, validate, and delete laio configurations."
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Creating Configurations

### New Configuration

Create a new configuration from the default template:

```bash
laio config create myproject
```

This creates `~/.config/laio/myproject.yaml` with:
- Editor window with `$EDITOR`
- Terminal window with two panes

### Copy From Existing

Create a new configuration by copying an existing one:

```bash
laio config create newproject --copy existingproject
```

This copies `existingproject.yaml` to `newproject.yaml`, updating the session name.

### Local Configuration

Create a `.laio.yaml` in the current directory:

```bash
laio config create
```

See [Local Configs](/docs/workflow/local-configs) for more details.

## Editing Configurations

### Edit in $EDITOR

```bash
laio config edit myproject
```

Opens `~/.config/laio/myproject.yaml` in your configured editor.

### Direct Editing

```bash
$EDITOR ~/.config/laio/myproject.yaml
```

## Validating Configurations

Validate before starting to catch errors:

```bash
laio config validate myproject
```

Validate a local configuration:

```bash
laio config validate --file .laio.yaml
```

Validation checks:
- YAML syntax correctness
- Required fields present
- Field types match schema
- Enum values are valid (e.g., `flex_direction`)

## Listing Configurations

### List All Configs

```bash
laio config list
```

Shows all configurations with active session markers (`*`).

### JSON Output

```bash
laio config list --json
```

Useful for scripting and automation.

### List Active Sessions Only

```bash
laio session list
```

Shows only currently running laio-managed sessions.

## Deleting Configurations

### With Confirmation

```bash
laio config delete myproject
```

Prompts for confirmation before deleting.

### Force Delete

```bash
laio config delete myproject --force
```

Deletes without confirmation.

**Note:** This only deletes the configuration file. If the session is active, stop it first:

```bash
laio stop myproject
laio config delete myproject --force
```

## Configuration Locations

### Global Configurations

Stored in `~/.config/laio/`:

```
~/.config/laio/
├── myproject.yaml
├── another-project.yaml
└── teamwork.yaml
```

### Local Configurations

Stored in project directories as `.laio.yaml`:

```
~/projects/myapp/
├── .laio.yaml
├── src/
└── package.json
```

### Linked Configurations

Local configs can be symlinked to global location:

```bash
cd ~/projects/myapp
laio config link myapp
```

Creates: `~/.config/laio/myapp.yaml` → `~/projects/myapp/.laio.yaml`

See [Local Configs](/docs/workflow/local-configs) for linking details.

## Common Workflows

### Template Configuration

Create a template for new projects:

```bash
laio config create template
laio config edit template
# ... customize as needed
```

Use it for new projects:

```bash
laio config create newproject --copy template
```

### Team Sharing

Share configurations via version control:

```bash
# In your project
laio config create
git add .laio.yaml
git commit -m "Add laio configuration"
git push
```

Team members can then:

```bash
git pull
laio start  # Automatically uses .laio.yaml
```

### Validation in CI

Add validation to CI pipeline:

```bash
#!/bin/bash
if [ -f .laio.yaml ]; then
  laio config validate --file .laio.yaml
fi
```

### Backup Configurations

```bash
# Backup all configs
cp -r ~/.config/laio ~/backups/laio-$(date +%Y%m%d)

# Backup single config
cp ~/.config/laio/myproject.yaml ~/backups/
```

## Configuration Schema

The JSON schema is available at `src/common/config/schema.json` in the repository.

For editor integration (LSP, validation), point your YAML language server to the schema.

Example for VS Code (`settings.json`):

```json
{
  "yaml.schemas": {
    "/path/to/laio-cli/src/common/config/schema.json": "*.laio.yaml"
  }
}
```
