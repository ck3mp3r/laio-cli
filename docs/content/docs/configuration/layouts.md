+++
title = "Layouts"
description = "Understanding flexbox-inspired layouts in laio."
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
+++

## Flexbox Concept

Laio uses a flexbox-inspired layout system similar to CSS flexbox. Panes are sized proportionally using the `flex` property and arranged using `flex_direction`.

## Flex Direction

`flex_direction` determines how panes are split:

- **`row`** (default): Vertical split line, panes side-by-side (left/right)
- **`column`**: Horizontal split line, panes stacked (top/bottom)

```yaml
windows:
  - name: example
    flex_direction: row  # vertical split, panes side-by-side
    panes:
      - flex: 1
      - flex: 1
```

## Flex Sizing

The `flex` property determines proportional sizing:

```yaml
panes:
  - flex: 1  # Takes 1 part
  - flex: 2  # Takes 2 parts (twice as large)
  - flex: 1  # Takes 1 part
# Result: 25% | 50% | 25%
```

### Equal Splits

```yaml
# Two equal panes (50/50)
panes:
  - flex: 1
  - flex: 1
```

### Asymmetric Splits

```yaml
# Sidebar + main (20/80)
panes:
  - flex: 1
  - flex: 4
```

## Nested Layouts

Panes can contain nested panes with their own `flex_direction`:

```yaml
windows:
  - name: dev
    flex_direction: row  # Vertical split: side-by-side
    panes:
      - flex: 2
        flex_direction: column  # Horizontal split: stacked
        panes:
          - flex: 3  # 75% of left side
          - flex: 1  # 25% of left side
      
      - flex: 1  # Right side: single pane
```

This creates:
```
┌─────────────┬──────┐
│             │      │
│   75%       │      │
│             │      │
├─────────────┤ 33%  │
│             │      │
│   25%       │      │
└─────────────┴──────┘
    66%
```

The outer `row` creates a vertical split (left|right).
The inner `column` creates a horizontal split (top/bottom) on the left side.

## Common Layouts

### Two-Column (IDE Style)

```yaml
windows:
  - name: workspace
    flex_direction: row
    panes:
      - flex: 1  # Sidebar/terminal
      - flex: 3  # Editor (3x larger)
```

### Three-Column

```yaml
windows:
  - name: workspace
    flex_direction: row
    panes:
      - flex: 1  # Left sidebar
      - flex: 2  # Main content
      - flex: 1  # Right sidebar
```

### Editor + Terminals Below

```yaml
windows:
  - name: dev
    flex_direction: column  # horizontal split: stacked
    panes:
      - flex: 3  # Editor (top 75%)
      - flex: 1
        flex_direction: row  # vertical split: side-by-side terminals
        panes:
          - flex: 1
          - flex: 1
```

Result:
```
┌─────────────────────┐
│                     │
│      Editor         │
│       75%           │
├──────────┬──────────┤
│          │          │
│   Term   │   Term   │
│   25%    │   25%    │
└──────────┴──────────┘
```

### Dashboard (4-pane Grid)

```yaml
windows:
  - name: dashboard
    flex_direction: row  # vertical split: two columns
    panes:
      - flex: 1
        flex_direction: column  # horizontal split: stacked
        panes:
          - flex: 1  # Top-left
          - flex: 1  # Bottom-left
      
      - flex: 1
        flex_direction: column  # horizontal split: stacked
        panes:
          - flex: 1  # Top-right
          - flex: 1  # Bottom-right
```

Result:
```
┌──────────┬──────────┐
│          │          │
│  TL      │  TR      │
│          │          │
├──────────┼──────────┤
│          │          │
│  BL      │  BR      │
│          │          │
└──────────┴──────────┘
```

## Focus Control

Set `focus: true` on a pane to place the cursor there on session start:

```yaml
panes:
  - flex: 1
    commands:
      - command: npm run build
  
  - flex: 2
    focus: true  # Cursor starts here
    commands:
      - command: npm run dev
```

**Note:** Only set `focus: true` on one pane. If multiple panes have focus, the last one wins.

## Zoom Control

Start a pane in zoomed state (hides other panes):

```yaml
panes:
  - flex: 1
  
  - flex: 2
    zoom: true  # Starts zoomed, hiding other panes
    commands:
      - command: tail -f logs/app.log
```

Use tmux `prefix + z` to toggle zoom manually.

## Limitations

There are practical limits to nesting depth. Very deep nesting (5+ levels) may cause layout issues. Test your configurations to find what works best.

For complex multi-pane setups, consider:
- Using multiple windows instead of deeply nested panes
- Keeping nesting to 2-3 levels maximum
- Testing layouts before committing to a workflow
