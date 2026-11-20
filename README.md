# DQM3 Synthesis Tool

Companion tool for **Dragon Quest Monsters: The Dark Prince**: a desktop
synthesis calculator (Rust + egui), then an MCP server over the same data.
Monster and talent names are shown in French and English.

## Getting started

The game data is a git submodule:

```bash
git clone --recursive <this-repo>
# already cloned without --recursive:
git submodule update --init
```

```bash
cargo build
cargo test
```

Run the calculator (release build: smaller, faster, no console window):

```bash
cargo run --release -p dqm3-gui
```

Or download `dqm3-gui.exe` (Windows, standalone) from the
[Releases](https://github.com/achachziad/DQM3_calculator/releases) page, or
from the latest run of the `build` workflow (Actions tab > Artifacts).
To publish a release: `git tag v0.1.0 && git push origin v0.1.0`.

Optional, manual: import French talent names (network, see `docs/DATA.md`):

```bash
cargo run -p dqm3-import
```

## Credits

- **Game data**: [MetalKid/DQM3_Database](https://github.com/MetalKid/DQM3_Database)
  (MIT license), by MetalKid — thank you! Also see https://dev.metalkid.info.
- **French talent names**: [wikidragonquest.fr](https://wikidragonquest.fr),
  the French Dragon Quest wiki, via its MediaWiki API. Their license is not
  stated, so this data is not redistributed in this repository.

Dragon Quest is a trademark of Square Enix. This is an unofficial fan project.
