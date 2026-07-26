# Imagyx

Imagyx est une bibliothèque d’images **locale, offline-first et accélérée automatiquement**. Elle suit les dossiers choisis, affiche les fichiers directement et génère des embeddings visuels sur la machine, puis permet de retrouver une image par son nom ou avec une description naturelle comme « fille tenant un téléphone ».

## Ce que contient le prototype

- application desktop Tauri 2 ;
- interface Vue 3 + Pinia + TypeScript + Vite ;
- icônes `@lucide/vue` ;
- design system local dans `src/components/ui` ;
- thèmes système, clair et sombre basés sur des variables CSS globales ;
- explorateur d’images avec dossiers suivis, grille et recherche globale ;
- téléchargement du modèle au lancement avec toast, progression en octets, Mo et pourcentage ;
- indexation incrémentale récursive ;
- lecture parallèle des métadonnées sans générer de JPEG dupliqués ;
- affichage paresseux et décodage asynchrone des fichiers originaux dans la grille ;
- recherche hybride nom de fichier + similarité CLIP ;
- embeddings image/texte FastEmbed exécutés localement avec ONNX Runtime ;
- CoreML sur macOS, DirectML sur Windows, CUDA optionnel sous Linux, puis CPU en repli silencieux ;
- SQLite en WAL pour les métadonnées et les vecteurs ;
- cache vectoriel en mémoire pour une recherche multithread rapide ;
- tests Rust et TypeScript, CI Linux/macOS/Windows.

Il n’existe aucun sélecteur de « mode performance ». Imagyx choisit le meilleur accélérateur disponible et retombe automatiquement sur le CPU si le provider GPU n’est pas utilisable.

## Design system

Les composants réutilisables sont regroupés dans `src/components/ui` :

```text
components/ui/
├── Badge/
├── Button/
├── Input/
├── Popover/
├── ProgressBar/
├── Select/
├── Skeleton/
├── Toast/
└── Tooltip/
```

Les couleurs, espacements, rayons, niveaux de texte et états sont définis dans `src/style.css` par des variables globales. Le thème clair est blanc, neutre et bleu SaaS ; le thème sombre utilise les mêmes rôles sémantiques. Aucun glow ou dégradé décoratif n’est utilisé.

## Téléchargement du modèle

Au lancement, Imagyx vérifie silencieusement son cache local. Si des fichiers CLIP manquent :

1. le backend récupère leur taille totale ;
2. le téléchargement s’exécute dans un worker Rust bloquant séparé ;
3. un événement Tauri remonte les octets téléchargés, le total, le fichier courant et le nombre de fichiers ;
4. le toast Vue affiche les Mo, le pourcentage et une barre de progression ;
5. les modèles sont initialisés avec l’accélérateur disponible ;
6. le toast confirme que l’IA locale est prête.

L’interface et la recherche par nom restent utilisables pendant cette préparation. Après le premier téléchargement, les modèles sont relus depuis le cache local.

## Indexation rapide

L’indexation est volontairement séparée en deux phases :

1. une seule lecture SQLite récupère les empreintes existantes du dossier ;
2. les dimensions et métadonnées des nouveaux fichiers sont lues en parallèle ;
3. SQLite est mis à jour immédiatement et la grille devient disponible ;
4. CLIP traite ensuite les images par paquets de 32 en arrière-plan ;
5. les vecteurs sont enregistrés progressivement.

La grille utilise directement les fichiers originaux avec `loading="lazy"` et `decoding="async"`. Imagyx ne crée donc plus une seconde copie JPEG de chaque image. Les anciens aperçus générés par le prototype sont supprimés automatiquement au prochain lancement.

Windows propose un cache Shell partagé et macOS fournit Quick Look Thumbnailing. Ces API seront surtout utiles quand Imagyx prendra en charge des formats que le WebView ne peut pas afficher directement, comme certains RAW ou documents. Pour les formats actuellement acceptés — JPEG, PNG, WebP, GIF, BMP et TIFF — l’affichage direct évite à la fois la duplication disque et une étape de génération supplémentaire.

## Stockage local

Toutes les données créées par Imagyx restent dans le dossier Images/Pictures de l’utilisateur :

```text
Pictures/
└── imagyx/
    ├── models/                  # modèles ONNX téléchargés au premier lancement
    ├── database/
    │   └── imagyx.sqlite3       # dossiers, images et embeddings
    ├── cache/
    │   └── thumbnails/          # dossier hérité, maintenu vide
    └── logs/
```

Les images originales ne sont ni copiées, ni modifiées, ni envoyées vers un serveur.

## Développement

### Prérequis

- Node.js 22.12 ou supérieur ;
- Rust 1.88 ou supérieur ;
- les dépendances système Tauri propres à la plateforme.

```bash
npm install
npm run tauri dev
```

### Vérifications

```bash
npm run typecheck
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

### Build desktop

```bash
npm run tauri build
```

## Accélération automatique

| Plateforme | Provider prioritaire | Repli |
| --- | --- | --- |
| macOS | CoreML | CPU ONNX Runtime |
| Windows | DirectML | CPU ONNX Runtime |
| Linux | CUDA avec `--features nvidia` | CPU ONNX Runtime |
| Autre | CPU ONNX Runtime | — |

Sous Linux NVIDIA :

```bash
npm run build
cargo build --release --manifest-path src-tauri/Cargo.toml --features nvidia
```

Le provider accéléré est enregistré avec un échec silencieux : un pilote absent ou incompatible ne bloque pas l’application.

## Architecture

```text
Vue / Pinia
    │ commandes et événements Tauri
    ▼
Rust
    ├── téléchargement hf-hub avec progression
    ├── indexeur métadonnées Rayon
    ├── FastEmbed + ONNX Runtime par lots
    ├── cache vectoriel en mémoire
    └── SQLite WAL
```

La recherche vectorielle reste volontairement en mémoire dans ce prototype. Pour une bibliothèque normale de designer, cela évite le coût et la complexité de distribution d’une extension SQLite native. Le schéma `embeddings` permet d’ajouter ultérieurement `sqlite-vec` ou `vec1` sans réindexer les fichiers.

## Limites actuelles du prototype

- le moteur sémantique est CLIP ViT-B/32, rapide mais moins précis qu’un grand VLM ;
- le renommage automatique n’est pas encore appliqué aux fichiers originaux ;
- les changements de dossiers sont détectés par une actualisation automatique périodique, avec réindexation manuelle immédiate disponible dans la barre latérale ;
- la signature/notarisation macOS et la signature Windows ne sont pas configurées ;
- l’exécution GPU doit encore être testée sur du matériel physique avant une distribution publique.
