# Migration Plan: Adopt nu-mcp Workflow Tools with Modern Single-Artifact Approach

## Current State Analysis

### laio-cli Current Workflow (GOOD - Keep This!)
- Clean single-artifact builds using redesigned rustnix `buildPackage`
- `nix build --system $target .#archive` creates system-named archives
- flake-parts with explicit system support (no x86_64-darwin)
- Supports 3 platforms: aarch64-darwin, aarch64-linux, x86_64-linux

### nu-mcp Workflow Tools We Want to Adopt
- Uses ck3mp3r/actions/nu-tools for release operations
- Simple 3-job workflow: prep-release, build, finalize-release
- Automatic version calculation and branch creation
- Automatic platform data generation
- Clean artifact handling with GitHub artifacts

### What We DON'T Want from nu-mcp
- Matrix build strategy (we have better single-artifact approach)
- Old rustnix `buildPackages` function (we use modern `buildPackage`)

## Migration Plan

### Phase 1: Adapt nu-tools to work with our single-artifact approach
1. **Keep our modern flake structure**
   - Continue using `buildPackage` from redesigned rustnix
   - Keep `nix build --system $target .#archive` approach
   - Keep flake-parts with explicit systems

2. **Align data directory structure**
   - Keep current `nix/data/` structure but ensure it matches nu-mcp pattern
   - Data files should contain `url` and `hash` fields

### Phase 2: Replace custom actions with nu-tools
1. **Remove custom GitHub actions**
   - Delete `.github/actions/version/`
   - Delete `.github/actions/package-data/`
   - Delete `.github/actions/release/`

2. **Update workflows to use nu-tools**
   - Replace release.yaml with nu-mcp pattern
   - Update CI workflow to be simpler
   - Use ck3mp3r/actions/nu-tools@main

### Phase 3: Handle Homebrew Formula updates
Since nu-mcp doesn't have Homebrew integration, we need to:
1. **Extend nu-tools or create custom step**
   - Add Homebrew formula update to finalize-release step
   - Update Formula/laio.rb with new version and hashes
   - Could be integrated into the commit-files step

2. **Alternative: Keep minimal custom action**
   - Create simple action just for Homebrew formula updates
   - Call it in finalize-release after nu-tools steps

## Implementation Steps

### Step 1: Move data files to root and ensure nu-tools compatibility
- [x] Move nix/data/*.json to data/*.json (no more nix files, no need for nesting)
- [x] Check current format vs nu-mcp data/*.json format (identical!)
- [x] Ensure data files have correct `url` and `hash` fields (they do)
- [x] Update flake.nix to reference ./data/ instead of ./nix/data/

### Step 2: Create new release workflow using nu-tools + our single-artifact approach
- [ ] Create new `.github/workflows/release.yaml` based on nu-mcp pattern
- [ ] Adapt build job to use `nix build --system $target .#archive` instead of matrix
- [ ] Use nu-tools for prep-release and finalize-release
- [ ] Add Homebrew formula update step in finalize-release

### Step 3: Create simple CI workflow
- [ ] Replace current complex test workflow with nu-mcp style
- [ ] Simple build test: `nix build .#laio`

### Step 4: Handle Homebrew integration
- [ ] Research nu-tools extensibility for Homebrew
- [ ] Implement Homebrew formula update in finalize-release
- [ ] Ensure Formula/laio.rb gets updated with correct version/hashes

### Step 5: Clean up old workflows
- [ ] Remove old release.yaml
- [ ] Remove custom actions directory
- [ ] Remove old test.yaml

## Expected Benefits

1. **Best of Both Worlds**: Modern single-artifact builds + proven nu-tools workflow
2. **Simplicity**: Much simpler workflow files using battle-tested nu-tools
3. **Consistency**: Same release process as other ck3mp3r projects (but modernized)
4. **Maintainability**: Less custom code to maintain
5. **Reliability**: nu-tools handles version management, artifact handling, etc.
6. **Future-Proof**: Our single-artifact approach can be adopted by other projects

## Risks and Mitigations

1. **nu-tools compatibility**: nu-tools expects matrix builds, we use single-artifact
   - Mitigation: Adapt the build step, nu-tools should handle artifacts the same way
2. **Homebrew integration**: nu-tools might not support this
   - Mitigation: Extend nu-tools or add custom step
3. **Different project structure**: laio has Formula/ directory and nix/data/
   - Mitigation: Adapt nu-tools patterns to work with laio structure

## Questions for Review

1. Should we extend nu-tools to support Homebrew, or add a custom step?
2. Do we want to keep the same 3-platform matrix, or adjust?
3. Should we move nix data files from `nix/data/` to `data/` to match nu-mcp?
4. Any other laio-specific requirements that need special handling?