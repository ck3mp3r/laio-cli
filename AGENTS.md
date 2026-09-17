# Development Rules for laio-cli

A simple, flexbox-inspired layout and session manager for tmux.

## Commands

```bash
cargo build --release
cargo test
cargo clippy --workspace --tests -- -D warnings   # warnings are errors
cargo fmt -- --check
```

- `cargo clippy --workspace --tests -- -D warnings` must pass before commit.
- `cargo test --workspace` must pass before commit.
- `cargo fmt -- --check` must pass before commit.

## Testing Philosophy

### Test-Driven Development (TDD)

**Always follow the RED → GREEN → REFACTOR cycle:**

1. **RED** - Write a failing test first.
2. **GREEN** - Write minimal code to make the test pass.
3. **REFACTOR** - Clean up and improve the code.

Never write production code without a failing test first.

### Test Organization

**No inline tests.** Tests must be in separate files in `src/`:

```
src/
  app/
    manager/
      config/
        manager.rs
        test.rs
      session/
        manager.rs
        test.rs
    cli/
      command_line.rs
      command_line_test.rs
  common/
    cmd/
      mod.rs
      test.rs
    config/
      template.rs
      template_test.rs
      variables.rs
      variables_test.rs
  muxer/
    tmux/
      mod.rs
      test.rs
    zellij/
      mod.rs
      test.rs
```

- Single-file module: `foo.rs` with sibling `foo_test.rs`.
- Multi-file module: `foo/mod.rs` with `foo/test.rs`.
- Declare test modules with `#[cfg(test)] mod test;` or `#[cfg(test)] mod foo_test;`.
- Forbidden: mixed `foo.rs` + `foo/test.rs`.

### No test-only code in production

**`#[cfg(test)]` on `fn` in production files is banned.**

- ❌ NO: `#[cfg(test)]` on any `fn` in production code (any `.rs` file that is not `*_test.rs` / `test.rs`)
- ✅ YES: `#[cfg(test)]` on `mod` declarations for test modules (e.g. `#[cfg(test)] mod test;`)
- ❌ NO: Test-only accessor methods that expose private fields solely for tests
- ✅ YES: Design public API so tests use the same methods as production code
- ✅ YES: Test through behavior — if the public API does not expose enough to verify a behavior, the API is missing a method, not the test a backdoor
- ❌ NO: Making a field `pub(crate)` just so tests can peek at internal state

**If a test needs to inspect private state, one of these is wrong:**

1. The test is testing implementation details — rewrite it to assert outcomes through the public API.
2. The design is wrong — the internal state should not matter, only its observable effects.

### Test Through the Public Boundary

- Tests verify behavior through the actual public API, not through internal types or methods exposed solely for testing.
- No private helper functions made public just for tests — shared logic is a private implementation detail, tested indirectly through observable output.
- If a test needs to inspect internal state, rewrite it to assert outcomes through the public API.

### What counts as production code

Any `.rs` file that is NOT a test file (`*_test.rs`, `test.rs`) is production code. This includes `mod.rs`, `lib.rs`, `main.rs`, `manager.rs`, etc.

### Mocking

**Use mocks for external dependencies.**

- Mock external processes (tmux, zellij, shell commands) via the `Runner`/`Cmd` traits in `src/common/cmd/`.
- Mock file system interactions via dependency injection.
- Prefer trait-based abstractions for mockable interfaces.

## Code Quality

- Write tests before implementation.
- Keep functions small and focused.
- Use meaningful names for tests (describe what they verify).
- Each test should verify one behavior.
- Refactor only when tests are green.
- No hidden global mutable state — use explicit state via structs.

### Error Handling

- Use `miette::{bail, Result}` and `miette!` macros for errors.
- Propagate errors with `?` instead of panicking.
- Use `.ok_or("message")?` or `.map_err(|e| miette!("...: {}", e))?` for conversions.

### Modern Rust (Edition 2024)

- Use if-let chains: `if let Some(x) = foo && x > 0 { ... }` — do NOT nest `if` inside `if`.
- Avoid `ref` bindings — use match ergonomics.
- Inline format args: `println!("{name}")` not `println!("{}", name)` for simple variables.
- Prefer `let ... else` for early returns over nested `if`.

### Nested `if` Statements

Flatten nested conditions. Never nest `if` inside `if` when they can be combined.

```rust
// ❌ WRONG
if let Some(x) = foo {
    if x > 0 {
        do_thing();
    }
}

// ✅ CORRECT
if let Some(x) = foo && x > 0 {
    do_thing();
}

// ✅ ALSO CORRECT — early return
let Some(x) = foo else { return; };
if x > 0 {
    do_thing();
}
```

### Module Files

Use `mod.rs` for module declarations and intentional re-exports. Put declarations and re-exports after any module-level documentation:

```rust
//! Module-level documentation.

mod event_base;
mod support;

pub use event_base::*;
```

Implementation details should normally live in dedicated source files rather than in `mod.rs`.

### `unwrap()` / `expect()` Policy

- ❌ NO: `unwrap()` or `expect("...")` in production code
- ❌ NO: `unwrap()` or `expect("...")` in test or example code
- ✅ YES: `unwrap_or`, `unwrap_or_else`, `if let`, `?`, `.ok_or("should have ...")?`
- ✅ YES: `mutex.lock().unwrap()` — mutex poison is a fatal internal inconsistency and panicking is correct
- For tests, use `.ok_or("should be ...")?` with `type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>`

### Scope Discipline

Do NOT refactor adjacent code "while you're at it." Every change must be scoped to the task at hand. If something else needs fixing, create a new task.

### `#[allow(...)]` Is Never a Fix

Suppressing a clippy warning is not fixing it. If clippy flags something, fix the underlying code. The only exception is `#[allow(dead_code)]` on test helpers that are intentionally unused — and even then, prefer deleting the dead code.

## Workflow

### No Parallel Developer Agents

**NEVER run two developer agents concurrently on the same repository.**

Compiled projects share a build cache, lock files, and the working tree. Parallel agents will corrupt each other's builds and produce interleaved file edits.

- Delegate one developer task at a time.
- Wait for it to complete before delegating the next.
- Researcher and reviewer agents may run in parallel with each other, but never alongside a developer.

### Review Before Commit

**Always run a review before `git commit`. No exceptions.**

```bash
cargo clippy --workspace --tests -- -D warnings
cargo test --workspace
cargo fmt -- --check
```

A reviewer subagent must sign off before committing. Clippy warnings are build failures.

### Git Workflow

Before ANY code changes:

1. Check branch, switch to `main` if needed.
2. Pull latest: `git pull`.
3. Create feature branch: `git checkout -b feature/name`.
4. Ask permission: state branch name and planned changes.
5. Never auto-commit — always ask first.

**NEVER commit to `main`/`master`.**

## Documentation

- Keep `README.md` aligned with user-visible behavior changes.
- Keep `docs/content/docs/` aligned with configuration and workflow changes.
- The YAML configuration schema is defined in `src/common/config/schema.json`. Update it when config fields change.
- If you change the CLI surface, update command examples in `README.md` and `docs/content/docs/reference/cli-commands.md`.

## Nix and Reproducible Builds

The project provides a Nix flake. Use `nix develop` for a reproducible development shell.

```bash
nix develop
direnv allow
```

Do not add build logic to `Makefile` unless it is related to GitHub Actions local testing via `act`.

## SOLID Principles

- Follow SOLID principles throughout:
  - **S**ingle Responsibility: One reason to change
  - **O**pen/Closed: Open for extension, closed for modification
  - **L**iskov Substitution: Subtypes must be substitutable
  - **I**nterface Segregation: Many specific interfaces over one general
  - **D**ependency Inversion: Depend on abstractions, not concretions
- Use trait-based abstractions for boundaries that external systems or tests must cross.
- Dynamic dispatch (`Box<dyn Trait>`) is permitted only at external boundaries where required (e.g., `Multiplexer` selection in `src/muxer/mod.rs`). Prefer static dispatch with generics internally.
