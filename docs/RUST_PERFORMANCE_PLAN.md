# Plan final — architecture Rust et performances

Ce document fixe l’architecture cible du backend Rust d’Imagyx et l’ordre des optimisations à mesurer. La base SQLite reste la source durable. Les index en mémoire sont des projections reconstruisibles.

## Architecture retenue

```text
src-tauri/src/
├── commands/       # surface Tauri, organisée par domaine
├── database/       # connexion, migrations, dossiers, assets, requêtes, embeddings
├── fuzzy/          # normalisation FTS et helpers de ranking
├── indexer/        # scan, métadonnées, progression et recherche hybride
├── tracing/        # spans, événements et p50/p95, uniquement en debug
├── vector_store/   # matrice contiguë, upsert incrémental et top-K
├── state.rs
└── lib.rs
```

Les fichiers métier doivent idéalement rester sous 200 lignes. Une exception est acceptable pour une implémentation SQL cohérente, mais pas pour mélanger plusieurs responsabilités.

## Changements intégrés dans la PR #1

- découpage de `commands.rs`, `db.rs` et `indexer.rs` en modules courts ;
- SQLite FTS5 avec tokenisation Unicode, suppression des diacritiques et index de préfixes ;
- requêtes dédiées pour les images en attente, les IDs demandés et les fingerprints d’un dossier ;
- suppression groupée des fichiers absents via une table temporaire et `DELETE ... RETURNING` ;
- `VectorStore` contigu, normalisé une fois, mis à jour par batch sans recharger tous les embeddings ;
- recherche vectorielle top-K avec heaps bornés et réduction Rayon, sans `HashMap` de 100 000 scores ni tri global ;
- fusion hybride par Reciprocal Rank Fusion sur au plus quelques centaines de candidats ;
- maximum de 60 résultats pour une recherche non vide ;
- chargement des vecteurs hors du chemin critique de démarrage : le lexical est disponible avant le sémantique ;
- tracing maison dans `tracing/`, compilé en mode no-op en release et actif seulement avec `debug_assertions` ;
- échantillons bornés et helpers p50/p95 pour les spans de recherche, indexation et startup ;
- tests unitaires pour le codec de vecteurs, FTS5, les requêtes pending, la normalisation fuzzy, le ranking, le top-K et les suppressions du store.

## Commandes de validation locale

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
npm run typecheck
npm test
npm run build
```

Le tracing s’affiche uniquement avec un build de développement, par exemple `npm run tauri dev`. Un build release utilise les helpers no-op de `tracing/release.rs`.

## Budgets de performance à mesurer

| Opération | Objectif initial à 100 000 images |
|---|---:|
| Résultats lexicaux chauds | p95 < 20 ms |
| Fusion sémantique chaude | p95 < 80 ms |
| Ouverture Spotlight | < 120 ms |
| Premier affichage utile | < 700 ms |
| Résultats renvoyés pour une requête | 60 maximum |
| Rescan inchangé | aucun décodage d’image |

Les mesures doivent être séparées pour l’ouverture SQLite, le chargement des métadonnées, le chargement des vecteurs, la recherche FTS, la recherche vectorielle et la fusion hybride.

## Étapes suivantes, déclenchées par les mesures

### 1. Benchmarks reproductibles

Ajouter des jeux synthétiques de 10 000, 50 000 et 100 000 images avec la dimension réelle de MobileCLIP-S0. Publier p50, p95, débit d’indexation et mémoire maximale.

### 2. Pipeline d’indexation borné

Faire évoluer l’indexation vers des channels bornés : discovery, metadata workers, decode/resize, batch GPU, writer SQLite unique, puis `VectorStore::upsert`. Le GPU doit traiter N pendant que le CPU prépare N+1 et que SQLite sauvegarde N-1.

### 3. Watcher événementiel

Remplacer le rescan de dossier par des commandes dédupliquées `Upsert`, `Remove` et `Rename`. Garder un rescan de réparation uniquement pour les événements perdus ou les incohérences.

### 4. Jobs persistants

Ajouter une table de jobs avec statut, modèle, tentatives et dernière erreur. Après crash, reprendre uniquement les images incomplètes.

### 5. Index vectoriel persistant

Persister la matrice contiguë avec un header versionné (`model_id`, dimensions, révision DB). Envisager f16 seulement après un test de qualité. Utiliser un fichier temporaire puis un renommage atomique.

### 6. HNSW seulement si nécessaire

Ajouter HNSW uniquement si le brute force contigu dépasse le budget p95 sur les machines cibles. Conserver le fallback brute force pendant la reconstruction.

### 7. Inférence native Rust

Évaluer ONNX Runtime/CoreML/DirectML/CUDA dans un chantier séparé après stabilisation de la recherche. Ne pas mélanger ce risque de packaging avec le travail FTS/vector store.
