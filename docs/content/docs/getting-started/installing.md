+++
title = "Installing"
description = "Installing laio."
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Supported flavors are Linux and Mac (aarch64 and x86_64)."
toc = true
top = false
+++

## Nix

```bash
nix profile install "github:ck3mp3r/laio-cli"
```

## Homebrew

```bash
brew tap ck3mp3r/laio-cli https://github.com/ck3mp3r/laio-cli/

brew install laio
```

## Download

Download the binary suitable for your system from the [Release Page](https://github.com/ck3mp3r/laio-cli/releases)
and place it in your `PATH`.
