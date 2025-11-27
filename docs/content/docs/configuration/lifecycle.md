+++
title = "Lifecycle Hooks"
description = "Manage session startup and shutdown with commands and scripts."
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Overview

Laio provides lifecycle hooks to run commands and scripts when sessions start or stop. This is useful for:

- Starting/stopping services (Docker, databases, etc.)
- Setting up environment state
- Cleaning up resources
- Initializing development tools

## Startup Hooks

### Startup Commands

Commands that run **before** the session is created:

```yaml
startup:
  - command: docker-compose
    args:
      - up
      - -d
  
  - command: npm
    args:
      - run
      - db:migrate
```

Commands execute sequentially in the order defined.

### Startup Script

A script that runs **after** startup commands:

```yaml
startup_script: |
  #!/usr/bin/env bash
  echo "Initializing session..."
  sleep 2
  curl http://localhost:8000/health
```

**Requirements:**
- Must include a shebang line (e.g., `#!/usr/bin/env bash`)
- Runs after all `startup` commands complete
- Blocks session creation until complete

## Shutdown Hooks

### Shutdown Commands

Commands that run when the session stops:

```yaml
shutdown:
  - command: docker-compose
    args:
      - down
  
  - command: echo
    args:
      - "Session stopped"
```

### Shutdown Script

A script that runs **after** shutdown commands:

```yaml
shutdown_script: |
  #!/usr/bin/env bash
  echo "Cleaning up..."
  rm -rf /tmp/session-*
  echo "Goodbye!"
```

**Requirements:**
- Must include a shebang line
- Runs after all `shutdown` commands complete
- Runs even if session is killed abruptly

## Execution Order

### On Session Start

1. `startup` commands execute (sequential)
2. `startup_script` executes
3. Windows and panes are created
4. Pane-level commands and scripts execute
5. Session is ready

### On Session Stop

1. Panes are destroyed
2. `shutdown` commands execute (sequential)
3. `shutdown_script` executes
4. Session is terminated

## Skipping Lifecycle Hooks

Skip startup hooks:
```bash
laio start myproject --skip-cmds
```

Skip shutdown hooks:
```bash
laio stop myproject --skip-cmds
```

## Pane-Level Scripts

In addition to session-level hooks, panes can have their own scripts:

```yaml
windows:
  - name: logs
    panes:
      - commands:
          - command: docker-compose
            args: [up, -d]
        
        script: |
          #!/usr/bin/env bash
          sleep 3
          docker-compose logs -f app
```

Pane scripts run **after** pane commands complete.

## Common Patterns

### Docker Development

```yaml
startup:
  - command: docker-compose
    args: [up, -d]

startup_script: |
  #!/usr/bin/env bash
  # Wait for services to be healthy
  timeout 30 bash -c 'until docker-compose exec db pg_isready; do sleep 1; done'
  echo "Database ready!"

shutdown:
  - command: docker-compose
    args: [down]
```

### Service Health Checks

```yaml
startup_script: |
  #!/usr/bin/env bash
  check_service() {
    until curl -s http://localhost:$1/health > /dev/null; do
      echo "Waiting for service on port $1..."
      sleep 1
    done
    echo "Service on port $1 is ready!"
  }
  
  check_service 8000
  check_service 3000
```

### Environment Setup

```yaml
startup_script: |
  #!/usr/bin/env bash
  # Load environment secrets
  if [ -f .env.local ]; then
    export $(cat .env.local | xargs)
  fi
  
  # Initialize tools
  direnv allow
  echo "Environment ready!"
```

### Cleanup on Exit

```yaml
shutdown_script: |
  #!/usr/bin/env bash
  # Clean temporary files
  rm -rf tmp/*
  rm -rf .cache/*
  
  # Reset git state (optional)
  git checkout -- .
  
  echo "Cleanup complete!"
```

## Error Handling

If a startup command or script fails:
- Session creation is aborted
- Any resources created are cleaned up
- Error message is displayed

If a shutdown command or script fails:
- Warning is displayed
- Session termination continues
- Resources may not be fully cleaned

## Best Practices

1. **Keep scripts idempotent** - Safe to run multiple times
2. **Add timeouts** - Prevent hanging on failed services
3. **Check for dependencies** - Verify tools exist before using them
4. **Use absolute paths** - Scripts run from session `path`
5. **Test separately** - Run scripts manually before using in configs
6. **Log output** - Echo progress for debugging

## Example: Full Lifecycle

```yaml
name: api-development
path: ~/projects/my-api

startup:
  - command: docker-compose
    args: [up, -d, postgres, redis]

startup_script: |
  #!/usr/bin/env bash
  set -e
  
  echo "Waiting for database..."
  timeout 30 bash -c 'until docker-compose exec -T postgres pg_isready; do sleep 1; done'
  
  echo "Running migrations..."
  npm run db:migrate
  
  echo "Seeding test data..."
  npm run db:seed
  
  echo "API environment ready!"

shutdown:
  - command: docker-compose
    args: [stop]

shutdown_script: |
  #!/usr/bin/env bash
  echo "Backing up logs..."
  cp logs/app.log logs/app.log.$(date +%Y%m%d-%H%M%S)
  
  echo "Stopping containers..."
  docker-compose down
  
  echo "Session ended."

windows:
  - name: api
    panes:
      - commands:
          - command: npm
            args: [run, dev]
```
