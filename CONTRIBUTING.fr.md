# Contribuer à Hypaper Engine

Merci de l'intérêt que vous portez au projet ! Ce document explique comment démarrer.

---

## Code de conduite

Ce projet suit le [Contributor Covenant](./CODE_OF_CONDUCT.fr.md).
En participant, vous vous engagez à respecter ce code.

---

## Comment contribuer

### Signaler un bug
Ouvrez une issue en utilisant le template **Bug Report**.
Merci d'inclure les étapes pour reproduire le bug, le comportement attendu, et votre environnement.

### Suggérer une fonctionnalité
Ouvrez une issue en utilisant le template **Feature Request**.
Pour toute modification du format `.hyscene`, ouvrez d'abord une discussion dans
[hypaper-spec](https://github.com/HypaperEngine/hypaper-spec).

### Soumettre une Pull Request
1. Forkez le dépôt
2. Créez une branche depuis `main` :
   ```bash
   git checkout -b feat/ma-fonctionnalite
   ```
3. Effectuez vos modifications
4. Vérifiez que tous les checks passent :
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   ```
5. Commitez en suivant les [Conventional Commits](https://www.conventionalcommits.org/) :
   ```
   feat(scene): ajout d'un nouveau type de calque
   fix(renderer): correction du calcul de blend mode
   ```
6. Ouvrez une Pull Request vers `main`

---

## Environnement de développement

### Prérequis
- Rust stable (voir `rust-toolchain.toml`)
- Compositeur Wayland (Hyprland recommandé)
- En-têtes de développement `libmpv`

### Compilation
```bash
git clone https://github.com/HypaperEngine/hypaper-engine
cd hypaper-engine
cargo build --all
```

### Lancer les tests
```bash
cargo test --all
```

---

## Nommage des branches

| Type | Préfixe | Exemple |
|---|---|---|
| Fonctionnalité | `feat/` | `feat/particle-system` |
| Correction de bug | `fix/` | `fix/shader-uniforms` |
| Maintenance | `chore/` | `chore/update-deps` |
| Refactoring | `refactor/` | `refactor/renderer-pipeline` |

---

## Convention de commits

Nous utilisons les [Conventional Commits](https://www.conventionalcommits.org/).
Scopes disponibles : `scene`, `renderer`, `wayland`, `hyprland`, `daemon`, `cli`, `types`

---

## Licence

En contribuant, vous acceptez que vos contributions soient publiées sous licence
[MIT](./LICENSE-MIT) OU [Apache-2.0](./LICENSE-APACHE).
