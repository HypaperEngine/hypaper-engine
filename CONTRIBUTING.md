# Contributing to Hypaper Engine

Thank you for your interest in contributing! This document explains how to get started.

---

## Code of Conduct

This project follows the [Contributor Covenant](./CODE_OF_CONDUCT.md).
By participating, you agree to uphold this code.

---

## How to Contribute

### Reporting a bug
Open an issue using the **Bug Report** template.
Please include steps to reproduce, expected behavior, and your environment.

### Suggesting a feature
Open an issue using the **Feature Request** template.
For changes to the `.hyscene` format, open a discussion in
[hypaper-spec](https://github.com/HypaperEngine/hypaper-spec) first.

### Submitting a Pull Request
1. Fork the repository
2. Create a branch from `main` :
   ```bash
   git checkout -b feat/my-feature
   ```
3. Make your changes
4. Make sure all checks pass :
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   ```
5. Commit using [Conventional Commits](https://www.conventionalcommits.org/) :
   ```
   feat(scene): add new layer type
   fix(renderer): correct blending calculation
   ```
6. Open a Pull Request against `main`

---

## Development Setup

### Prerequisites
- Rust stable (see `rust-toolchain.toml`)
- Wayland compositor (Hyprland recommended)
- `libmpv` development headers

### Build
```bash
git clone https://github.com/HypaperEngine/hypaper-engine
cd hypaper-engine
cargo build --all
```

### Run tests
```bash
cargo test --all
```

---

## Branch Naming

| Type | Pattern | Example |
|---|---|---|
| Feature | `feat/` | `feat/particle-system` |
| Bug fix | `fix/` | `fix/shader-uniforms` |
| Maintenance | `chore/` | `chore/update-deps` |
| Refactor | `refactor/` | `refactor/renderer-pipeline` |

---

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/).
Available scopes : `scene`, `renderer`, `wayland`, `hyprland`, `daemon`, `cli`, `types`

---

## Licence

By contributing, you agree that your contributions will be licensed under
[MIT](./LICENSE-MIT) OR [Apache-2.0](./LICENSE-APACHE).
