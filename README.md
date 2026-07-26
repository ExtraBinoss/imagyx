# Imagyx

Imagyx est une bibliothèque d’images locale, offline-first et conçue pour retrouver rapidement des visuels par nom ou par description naturelle.

## Prototype actuel

- application desktop Tauri 2 ;
- Vue 3, Pinia, TypeScript et Vite ;
- grille virtualisée avec miniatures générées uniquement pour les cartes visibles ;
- cache disque LRU limité à 256 miniatures et cache mémoire limité à 512 URL ;
- watcher natif récursif Windows, macOS et Linux ;
- SQLite en WAL pour les métadonnées et les embeddings ;
- MobileCLIP2-S0 exécuté directement avec ONNX Runtime ;
- téléchargement au premier lancement de `visual.onnx`, `visual.onnx.data`, `text.onnx`, `text.onnx.data` et `tokenizer.json` ;
- essai explicite de CoreML, DirectML ou CUDA, puis création de sessions CPU séparées si l’accélérateur échoue ;
- prétraitement parallèle des images vers 256 × 256 ;
- lots adaptatifs : 48 images avec accélération GPU confirmée, 12 images sur CPU ;
- progression et statistiques dans la sidebar avec popover détaillé ;
- informations CPU/RAM du processus via `sysinfo`.

Les poids ONNX utilisés par l’application proviennent du dépôt communautaire `RuteNL/MobileCLIP2-S0-OpenCLIP-ONNX`, exporté depuis MobileCLIP2-S0. Les anciens embeddings CLIP ViT-B/32 sont invalidés automatiquement lors de la migration afin d’éviter de mélanger deux espaces vectoriels différents.

## Sidebar IA

Le panneau compact de la sidebar affiche :

- téléchargement ou progression de l’analyse ;
- backend réellement initialisé ;
- indication `GPU actif`, `CPU` ou `CPU (repli)` ;
- taille du lot ;
- débit en images par seconde ;
- temps moyen par image ;
- consommation CPU et mémoire du processus ;
- dernière erreur du runtime, lorsqu’elle existe.

## Développement

Prérequis : Node.js 22.12+, Rust 1.88+ et les dépendances système Tauri de la plateforme.

```bash
npm install
npm run tauri dev
```

Vérifications locales recommandées :

```bash
npm run typecheck
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
cargo check --manifest-path src-tauri/Cargo.toml
```

GitHub Actions est volontairement configuré avec `workflow_dispatch` uniquement. Aucun commit ou pull request ne déclenche automatiquement un workflow.

## Stockage

```text
Pictures/imagyx/
├── models/
├── database/imagyx.sqlite3
├── cache/thumbnails/
└── logs/
```

Les fichiers originaux ne sont ni copiés, ni modifiés, ni envoyés vers un serveur.

## Limites

- cette migration MobileCLIP n’a pas été compilée dans l’environnement de l’assistant ;
- l’export ONNX est communautaire et doit être validé sur Windows et macOS ;
- l’activation GPU doit être testée sur du matériel réel ;
- la signature et la notarisation ne sont pas configurées.
