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

## Project Based Or Local Configurations

Not all laio configurations need to be stored in `~/.config/laio`, they can also reside in a project directory.
When you run `laio start` within a directory, laio will look for a file named `.laio.yaml` and also search up until it reaches the users home directory.

The only caveat using this approach is that 
```bash
laio ls
```
will not list project based laio configurations. 
If you want a project based laio configuration to be visible to laio during normal operation, you can link the local `.laio.yaml` using 
```bash
laio config link <name> 
```
within the directory it is located in, 
after which it will be linked via a symbolic link into `~/.config/laio/<name>.yaml` and subsequent 
```bash
laio start <name>
```
or 
```bash
laio ls 
```
will also pick up on the configuration.

## Saving Existing TMUX Sessions

Alternatively to creating new configurations manually or via 
```bash
laio config create <name> 
```
you can also use laio to create a config file from within the tmux session you are already in.
```bash
laio session yaml > ~/.config/laio/<name>.yaml
```
This will serialise the current tmux session into the right format and into the file specified.
