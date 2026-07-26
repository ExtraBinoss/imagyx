# Imagyx

Imagyx est une bibliothèque d’images **locale, offline-first et accélérée automatiquement**. Elle suit les dossiers choisis, affiche rapidement les nouveaux fichiers et génère des embeddings visuels sur la machine, puis permet de retrouver une image par son nom ou avec une description naturelle comme « fille tenant un téléphone ».

## Ce que contient le prototype

- application desktop Tauri 2 ;
- interface Vue 3 + Pinia + TypeScript + Vite ;
- icônes `@lucide/vue` ;
- design system local dans `src/components/ui` ;
- thèmes système, clair et sombre basés sur des variables CSS globales ;
- explorateur d’images avec dossiers suivis, grille et recherche globale ;
- grille virtualisée : seules les cartes visibles et trois rangées d’overscan sont montées ;
- watcher natif récursif Windows, macOS et Linux avec debounce ;
- miniatures demandées uniquement par les cartes visibles ;
- quatre générations de miniatures simultanées au maximum ;
- cache disque LRU borné à 256 miniatures et cache mémoire borné à 512 URL ;
- téléchargement du modèle au lancement avec toast, progression en octets, Mo et pourcentage ;
- indexation incrémentale récursive ;
- lecture parallèle des métadonnées ;
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

L’interface et la recherche par nom restent utilisables pendant cette préparation. Les fichiers découverts sont immédiatement visibles. Leur analyse sémantique est mise en file d’attente et démarre automatiquement dès que le modèle est prêt.

## Indexation rapide

L’indexation est séparée en phases indépendantes :

1. une seule lecture SQLite récupère les empreintes existantes du dossier ;
2. les dimensions et métadonnées des nouveaux fichiers sont lues en parallèle ;
3. SQLite est mis à jour et la grille devient disponible ;
4. si le modèle est encore en préparation, l’analyse IA est indiquée comme mise en attente sans bloquer l’interface ;
5. CLIP traite ensuite les images par lots de huit ;
6. la barre d’état affiche le lot courant, le nombre total de lots et le nombre exact d’images terminées ;
7. les vecteurs sont enregistrés après chaque lot.

Le premier lot utilise une progression indéterminée avec un libellé comme `Lot 1 / 10`, au lieu d’afficher un `0 / 76` immobile. Après le premier lot, la progression devient exacte.

## Virtualisation et miniatures

La grille calcule le nombre de colonnes à partir de la largeur disponible et ne rend que les rangées présentes dans le viewport, avec trois rangées supplémentaires avant et après. Une bibliothèque de plusieurs milliers d’images ne crée donc pas plusieurs milliers de nœuds DOM ni plusieurs milliers de requêtes simultanées.

Lorsqu’une carte devient visible :

1. Vue place sa demande dans une file bornée ;
2. quatre demandes peuvent être exécutées simultanément ;
3. Rust retourne immédiatement une miniature déjà présente dans le cache, ou en génère une de 512 px ;
4. le cache disque élimine les entrées les moins récemment utilisées au-delà de 256 fichiers ;
5. une file frontend supérieure à 96 demandes abandonne les anciennes demandes devenues hors écran.

Cette stratégie évite de dupliquer toutes les images. Seule une petite fenêtre de miniatures utiles est conservée.

## Surveillance des dossiers

Chaque dossier suivi est enregistré auprès du watcher natif de la plateforme en mode récursif. Les créations, modifications, suppressions, renommages et nouveaux sous-dossiers déclenchent une réindexation incrémentale après un debounce de 700 ms. Les événements provenant du dossier interne `Pictures/imagyx` sont ignorés afin que les miniatures et la base de données ne créent pas de boucle.

Un dossier ajouté ou retiré depuis l’interface est ajouté ou retiré du watcher immédiatement, sans redémarrer l’application.

## Stockage local

Toutes les données créées par Imagyx restent dans le dossier Images/Pictures de l’utilisateur :

```text
Pictures/
└── imagyx/
    ├── models/                  # modèles ONNX téléchargés au premier lancement
    ├── database/
    │   └── imagyx.sqlite3       # dossiers, images et embeddings
    ├── cache/
    │   └── thumbnails/          # maximum 256 aperçus JPEG de 512 px
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
    ├── grille virtualisée
    ├── file de miniatures bornée
    └── événements Tauri
              ▼
Rust
    ├── watcher natif récursif
    ├── cache de miniatures LRU
    ├── téléchargement hf-hub avec progression
    ├── indexeur métadonnées Rayon
    ├── FastEmbed + ONNX Runtime par lots
    ├── cache vectoriel en mémoire
    └── SQLite WAL
```

La recherche vectorielle reste volontairement en mémoire dans ce prototype. Pour une bibliothèque normale de designer, cela évite le coût et la complexité de distribution d’une extension SQLite native. Le schéma `embeddings` permet d’ajouter ultérieurement `sqlite-vec` ou `vec1` sans réindexer les fichiers.

## Limites actuelles du prototype

- le moteur sémantique est CLIP ViT-B/32, rapide mais moins précis qu’un grand VLM ;
- la liste de métadonnées chargée par Vue est plafonnée à 20 000 résultats par vue, tandis que le DOM reste virtualisé ;
- le renommage automatique n’est pas encore appliqué aux fichiers originaux ;
- la signature/notarisation macOS et la signature Windows ne sont pas configurées ;
- l’exécution GPU doit encore être testée sur du matériel physique avant une distribution publique.
