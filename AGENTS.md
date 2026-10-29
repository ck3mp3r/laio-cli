# Development Rules for laio-cli

A simple, flexbox-inspired layout and session manager for tmux.

## Commands

```bash
cargo build --release
cargo nextest run
cargo clippy --workspace --tests -- -D warnings   # warnings are errors
cargo fmt -- --check
```

- `cargo clippy --workspace --tests -- -D warnings` must pass before commit.
- `cargo nextest run` must pass before commit.
- `cargo fmt -- --check` must pass before commit.

The test runner is cargo-nextest. Configuration lives in `.config/nextest.toml`. Each test runs in its own process.

## Testing Philosophy

### Test-Driven Development (TDD)

**Always follow the RED → GREEN → REFACTOR cycle:**

1. **RED** - Write a failing test first.
2. **GREEN** - Write minimal code to make the test pass.
3. **REFACTOR** - Clean up and improve the code.

Never write production code without a failing test first.

### Test Organization

**No inline tests.** Tests live in a top-level `test/` directory that mirrors the `src/` hierarchy:

```
src/                          test/
  app/                          app/
    manager/                      manager/
      session/                      session/
        manager.rs                    test/
        mod.rs                          stop.rs
                                        start.rs
                                        support.rs
  common/                       common/
    cmd/                          cmd/
      mod.rs                        test.rs
      shell_runner.rs               shell_runner_test.rs
    config/                       config/
      fixtures/                     template_test.rs
      mod.rs                        variables_test.rs
      template.rs
      variables.rs
  muxer/                        muxer/
    mod.rs                        test.rs
    herdr/                        herdr/
      mod.rs                        client_test/
                                      protocol.rs
                                      workspace.rs
                                    support/
                                      env.rs
                                      models.rs
    tmux/                         tmux/
      mod.rs                        test/
                                      client.rs
                                      mux_start.rs
```

- One test file per production module: `src/foo.rs` ↔ `test/foo.rs`; `src/foo/mod.rs` ↔ `test/foo/test.rs`.
- No test files in `src/` — no `*_test.rs` or `test.rs` next to production code.
- No `mod.rs` inside `test/`.
- Declare every test file from its production module with `#[cfg(test)]` plus `#[path]`:

```rust
// src/foo.rs
#[cfg(test)]
#[path = "../test/foo.rs"]
mod foo_test;
```

```rust
// src/foo/mod.rs
#[cfg(test)]
#[path = "../../test/foo/test.rs"]
mod foo_test;
```

- `#[path]` resolves relative to the directory of the declaring file, so it must escape `src/`. Deeper modules add one `../` per level — for example, `src/app/manager/config/mod.rs` uses `#[path = "../../../../test/app/manager/config/test.rs"]`.
- The `mod` declaration name is independent of the filename: the examples above declare `foo_test` for `test/foo.rs`.
- Test support files also live in `test/`, for example `test/foo/support.rs`, and are declared as their own `#[cfg(test)]` plus `#[path]` modules.
- Test file size limit: 500 lines. An over-limit file splits into topical subfiles in a same-named subdirectory, with shared fixtures in that subdirectory's `support.rs`. Declare each split file with its own `#[cfg(test)]` plus `#[path]`.

### Test Categorization

All current tests are unit tests.

The herdr tests bind local unix domain sockets to exercise the client wire protocol. That is fast, local, and deterministic, so those tests are unit tests.

IF a future test needs a real server, network I/O, or is inherently slow, THEN it is an integration test: gate it behind `#[cfg(all(test, feature = "integration"))]`, add an `integration` cargo feature, and add an `integration` nextest profile. None of these exist today.

### No test-only code in production

**`#[cfg(test)]` on `fn` in production files is banned.**

- ❌ NO: `#[cfg(test)]` on any `fn` in production code (any `.rs` file that is not in the `test/` directory)
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

Any `.rs` file that is NOT in the `test/` directory is production code. This includes `mod.rs`, `lib.rs`, `main.rs`, `manager.rs`, etc.

### Mocking

**Use mocks for external dependencies.**

- Mock external processes (tmux, zellij, shell commands) via the `Runner`/`Cmd` traits in `src/common/cmd/`.
- Mock file system interactions via dependency injection.
- Prefer trait-based abstractions for mockable interfaces.

### Code Coverage

Code coverage is measured with `cargo-llvm-cov` (LLVM source-based instrumentation, stable Rust).

Run it with the `coverage` wrapper in the nix dev shell. It runs `cargo llvm-cov nextest --workspace --cargo-profile coverage --html --output-dir coverage/ --ignore-filename-regex '(/test/|/test$)'`.

- Coverage is a second quality dimension next to clippy: clippy finds suspicious code, coverage finds code no test executes.
- Test files live in `test/`, which the default exclusion patterns do not cover — the `--ignore-filename-regex` flag keeps them out of the report.
- The `[profile.coverage]` build profile lives in `.cargo/config.toml` (`inherits = "test"`, `debug = "line-tables-only"`, `opt-level = 0`) for faster instrumented builds.
- The dev shell sets `LLVM_COV` and `LLVM_PROFDATA` to the nixpkgs `llvmPackages_23` binaries, matching the LLVM version rustc uses. Mismatched LLVM tool versions fail with raw profile version errors.
- `--fail-under-lines <N>` exits non-zero when total line coverage drops below `N` percent. No threshold is enforced yet.

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
cargo nextest run
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
