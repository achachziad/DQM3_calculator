# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# DQM3 Synthesis Tool

## État actuel du dépôt

Existe : workspace Cargo, `crates/dqm3-import`, `crates/dqm3-core` (phase 1
faite : 18 tables MetalKid embarquées, `Database` = point d'entrée unique), `data/` (sous-module),
`docs/DATA.md`, `README.md` (crédits). **Pas encore** : `dqm3-gui`, `dqm3-mcp`.
Nouveau crate : `cargo new --vcs none` (sinon `.git` imbriqué).
L'arborescence ci-dessous est la **cible** ; vérifie ce qui existe avant d'agir.
La racine du workspace est la racine du dépôt (`DQM3_calculator/`).

## Contexte

Outil compagnon pour **Dragon Quest Monsters: The Dark Prince** (DQM3), utilisé
**à côté du jeu** sur Windows.

1. **D'abord** : un calculateur de synthèse en application de bureau (egui).
2. **Ensuite** : un serveur MCP pour qu'une IA interroge les mêmes données.

Projet d'apprentissage : l'utilisateur est un ingénieur logiciel junior qui
prend la main avec Rust et le travail avec l'IA.
→ **Explique les choix Rust et leur principe** (ownership, lifetimes,
traits) quand tu les utilises.

## Langue

- **Conversation : français.**
- **Code, noms, commentaires, commits : anglais.**
- Données affichées : noms **FR et EN** côte à côte.

## Contraintes clés

- **Légèreté mémoire** : l'appli tourne en même temps que le jeu.
  Évite les dépendances lourdes. Justifie chaque nouvelle crate.
- **Windows** en priorité.
- **Local uniquement** pour l'instant (pas de serveur distant, pas de réseau).
  Seule exception : `dqm3-import`, outil lancé **à la main**, jamais par l'appli.

## Architecture

Workspace Cargo, **un cœur + des adaptateurs fins** :

```
DQM3_calculator/
├── Cargo.toml          # workspace
├── crates/
│   ├── dqm3-core/      # lib : modèles, chargement, index, requêtes
│   ├── dqm3-gui/       # bin : calculateur egui (eframe)
│   ├── dqm3-mcp/       # bin : serveur MCP (phase 3)
│   └── dqm3-import/    # bin : import manuel des noms FR des talents (réseau)
├── data/               # sous-module git MetalKid ; JSON dans data/data/json/raw/
├── data-fr/            # généré par dqm3-import, ignoré par git
└── docs/
    └── DATA.md         # schéma des données (LIRE avant de toucher aux modèles)
```

### Règles d'architecture (non négociables)

- `dqm3-core` **ne dépend ni d'egui ni de MCP**. Aucune notion d'interface.
- **Toute la logique métier vit dans `dqm3-core`.** Les adaptateurs ne font
  que convertir entrées/sorties et appeler le cœur.
- Une fonctionnalité = **une fonction du cœur**, réutilisée par la GUI **et**
  par MCP. Jamais de logique dupliquée.
  - Exemple : l'outil MCP `search_monsters` appelle `core::search_monsters()`.
- `dqm3-core` : pas de `unwrap()` / `expect()` hors tests. Erreurs typées.
- `dqm3-import` est **hors de l'appli** : ni `dqm3-core` ni `dqm3-gui` ne
  dépendent de lui ni de `ureq`. Le cœur ne lit que ses **fichiers de sortie**.

## Stack

| Rôle | Choix | Statut |
|---|---|---|
| Langage | Rust stable | ✅ |
| GUI | `eframe` / `egui` | ✅ |
| Désérialisation | `serde`, `serde_json` | ✅ |
| MCP | `rmcp` (SDK officiel, 2.x) | ⏳ phase 3 |
| HTTP (`dqm3-import` seulement) | `ureq` 3 (bloquant, sans runtime async) | ✅ |
| Tests | `cargo test` | ✅ |

⚠️ **Les API d'egui et de rmcp évoluent vite.** Avant de les utiliser :
vérifie la version réellement installée et sa documentation.
**N'invente jamais une API.** En cas de doute, dis-le.

## Données

Détail complet dans **`docs/DATA.md`**. L'essentiel :

- **MetalKid** (`data/`, sous-module, licence **MIT**, crédit requis dans le
  README) : monstres, talents, traits, lieux, recettes. Charger les
  **`raw/`** (tables liées par id), pas les `computed/`. Fichiers en UTF-8
  **avec BOM**. JSON ≈ 3,7 Mo → `include_str!` OK.
- **Noms FR** : monstres seulement chez MetalKid (`FrenchName`). Les
  **talents FR** viennent de **wikidragonquest.fr** via `dqm3-import`, à
  partir de la page « Liste des talents de DQM3 » (241 talents). Le **nom
  en jeu = libellé de la liste** (`Espoir de Flamme`), pas le titre de la
  page (`Apprenti de Flamme`, gardé en alias `fr_page`).
  Licence du wiki non précisée → `data-fr/` ignoré par git, **pas embarqué**
  tant que ce n'est pas clarifié.
- Pièges : 80 « monstres » sans `Number` = règles « famille + rang »
  (`Slime Family (G)`) ; recettes à 2 parents **ou** 4 grands-parents ;
  `IsPrimary` = talent fixe (3 anomalies dans la source).

## Fonctionnalités

### Phase 1–2 — Cœur + calculateur (V1)

**Liste des monstres**
- Recherche par nom **FR ou EN**, insensible à la casse **et aux accents**.
- Filtres : rang, famille, talent, taille.
- Tri : n°, rang, stats.

**Fiche monstre**
- Noms FR/EN, n°, famille, rang, stats max.
- Talents (fixe / aléatoires), traits (avec niveau de déblocage).
- Lieux de recrutement (zone, saison, météo).
- **Recettes** pour l'obtenir.
- **Synthèse inversée** : ce qu'on peut créer avec ce monstre.

**Talents**
- Paliers (compétence + points), évolution, monstres qui l'ont.

### Phase 3 — MCP

Outils prévus (chacun = un appel au cœur) :

| Outil MCP | Fonction du cœur |
|---|---|
| `search_monsters` | `search_monsters(filters)` |
| `get_monster` | `get_monster(id_or_name)` |
| `get_recipes` | `recipes_for(monster)` |
| `get_children` | `children_of(monster)` |
| `search_talents` | `search_talents(query)` |
| `translate` | `translate(name)` |

Transport : **stdio**, branché sur Claude Desktop.

### Plus tard (V2, hors périmètre actuel)

- Arbre complet jusqu'aux monstres recrutables.
- Planificateur de talents (« X avec tel et tel talent »).

## Méthode de travail

1. **Contexte d'abord** : s'il manque une info, **pose la question**
   au lieu de deviner.
2. **TDD** : écris le test, vérifie qu'il échoue, puis le code.
   - Tests du cœur avec de **petits jeux de données** (fixtures),
     pas le JSON complet.
   - Cas obligatoires : nom FR, nom EN, accents, nom introuvable,
     monstre sans recette (recrutable uniquement).
3. **Petits pas** : une fonctionnalité à la fois, qui compile et passe
   les tests avant la suivante.
4. **Ne réécris pas plus que nécessaire.**

## Commandes

```bash
cargo build                     # tout le workspace
cargo test                      # tous les tests
cargo test -p dqm3-core         # cœur uniquement
cargo test -p dqm3-core accent  # tests dont le nom contient « accent »
cargo test -p dqm3-core -- --exact search::tests::finds_by_fr_name  # un test précis
cargo run -p dqm3-gui           # lancer le calculateur
cargo run -p dqm3-import        # MANUEL, réseau : seulement à la demande de l'utilisateur
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt
git submodule update --init     # si data/ est vide après un clone
```

Ne lance **jamais** `dqm3-import` (ni de requête vers wikidragonquest.fr)
sans accord explicite : serveur mutualisé fragile (508 fréquents). Les tests
de `dqm3-import` sont hors ligne.

**Avant de dire « terminé »** : `cargo fmt`, `cargo clippy` sans
warning, `cargo test` vert.

## Roadmap

- [x] **Phase 0** — Explorer `data/`, écrire `docs/DATA.md`
- [x] **Phase 0b** — Import des talents FR (`dqm3-import`) : 241/241
      appariés, 239 paliers identiques (voir `docs/DATA.md` §2.5).
      Corrections manuelles dans `crates/dqm3-import/src/manual.rs`.
- [x] **Phase 1** — `dqm3-core` : modèles + chargement + index (+ tests)
- [ ] **Phase 2a** — `dqm3-core` : recherche, filtres, tris, recettes,
      synthèse inversée (+ tests)
- [ ] **Phase 2b** — `dqm3-gui` : liste filtrable + fiche monstre
- [ ] **Phase 3** — `dqm3-mcp` + branchement Claude Desktop
- [ ] **Phase 4** — V2 (arbre complet, planificateur de talents)