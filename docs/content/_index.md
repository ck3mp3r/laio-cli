+++
title = "laio - a simple, flexbox-inspired, layout & session manager for tmux."
description = "Welcome to laio."
sort_by = "weight"
weight = 1
template = "index.html"

# The homepage contents
[extra]
lead = '<img src="./media/laio.svg" width="450" />'
url = "/docs/getting-started/installing"
url_button = "Get started"
repo_version = "GitHub v0.15.0"
repo_license = "Apache License."
repo_url = "https://github.com/ck3mp3r/laio-cli"

# Menu items
[[extra.menu.main]]
name = "Docs"
section = "docs"
url = "/docs/getting-started/installing"
weight = 10

[[extra.list]]
title = "Flexbox-Inspired Layouts"
content = 'Define complex multi-pane layouts with intuitive row/column flex directions and proportional sizing.'

[[extra.list]]
title = "Session Lifecycle"
content = 'Manage startup/shutdown hooks with commands and embedded scripts for complete session control.'

[[extra.list]]
title = "Dual Config Modes"
content = 'Use global configs (~/.config/laio) or project-local .laio.yaml files for flexible workflows.'

[[extra.list]]
title = "Session Export"
content = 'Serialize existing tmux sessions to YAML format for sharing and templating.'

[[extra.list]]
title = "Supported Platforms"
content = 'Mac & Linux, x86 & arm64'

[[extra.list]]
title = "Built with Nix"
content = '<a href="https://builtwithnix.org/" target="_blank" /><img src="https://builtwithnix.org/badge.svg" /></a>'

+++
