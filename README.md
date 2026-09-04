# degunk

<div align="center">
  <h3>⚡ Blazing-fast disk cleaner & developer artifact scavenger for multi-stack codebases</h3>
  <p>Reclaim hundreds of gigabytes occupied by abandoned dependencies, build artifacts, test runs, and bloated global tool caches.</p>
</div>

---

## Highlights

* **Project Tree Hierarchy**: Multi-artifact projects (such as monorepos, fullstack apps, and mobile projects) group naturally under their root directory with expand/collapse (`Enter`/`e`/`E`).
* **Broad Stack Support**: Discovers artifacts across Node.js, Python, Rust, Java/Kotlin, Go, .NET, C/C++, Swift/iOS, Flutter/Dart, PHP, Elixir, Zig, Godot, Unity, Ruby, Scala, Haskell, OCaml, Terraform/IaC, and Coverage/Testing.
* **Global Developer Caches**: Dedicated tab (`Tab`) scans machine-level caches for Cargo, npm, pnpm, Yarn, pip, uv, Gradle daemons, Maven, Go, Pub, Bun, Ollama weights, LM Studio models, HuggingFace, PyTorch, Android, Xcode DerivedData, CocoaPods, Coursier, Ruby gems, Haskell Stack, Terraform plugin cache, JetBrains, VS Code, and Cursor.
* **Git Activity Tracking**: Automatic inspection of local Git history shows days since last commit and whether the working copy has uncommitted changes.
* **Smart Search Filtering**: Fast multi-field fuzzy search with structured filter tokens (`eco:rust`, `size:>100m`, `locked:yes`, `git:clean`).
* **Safe Deletion**: Recycle Bin / Trash support by default with read-only file permission handling on Windows, macOS, and Linux.

---

## Supported Ecosystems & Targets

| Ecosystem | Target Folders | Required Manifests | Lockfiles | Reinstall Command |
| :--- | :--- | :--- | :--- | :--- |
| **Node.js / Web** | `node_modules`, `.next`, `.nuxt`, `.turbo`, `.svelte-kit`, `.angular`, `.astro`, `.parcel-cache`, `.vite`, `.docusaurus`, `storybook-static` | `package.json`, `next.config.*`, `nuxt.config.*`, `angular.json`, `astro.config.*`, etc. | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` | `npm install` / `pnpm install` / `bun install` |
| **Python** | `.venv`, `venv`, `env`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `dist`, `build`, `*.egg-info`, `.tox`, `.nox`, `.pixi`, `.ipynb_checkpoints` | `pyproject.toml`, `requirements.txt`, `Pipfile`, `setup.py`, `pixi.toml` | `poetry.lock`, `Pipfile.lock`, `pdm.lock`, `uv.lock`, `pixi.lock` | `pip install -r requirements.txt` / `uv sync` |
| **Rust** | `target` | `Cargo.toml` | `Cargo.lock` | `cargo build` |
| **Java / Kotlin** | `build`, `.gradle`, `target` | `build.gradle`, `build.gradle.kts`, `pom.xml`, `gradlew` | `gradle.lockfile` | `./gradlew build` / `mvn clean install` |
| **Ruby** | `.bundle`, `vendor` | `Gemfile` | `Gemfile.lock` | `bundle install` |
| **Scala** | `target`, `.bloop`, `.metals` | `build.sbt` | N/A | `sbt compile` |
| **Haskell** | `.stack-work`, `dist-newstyle` | `stack.yaml`, `*.cabal`, `cabal.project` | N/A | `stack build` / `cabal build` |
| **OCaml** | `_build` | `dune-project`, `dune` | `dune.lock` | `dune build` |
| **Terraform / IaC** | `.terraform`, `.serverless`, `.aws-sam` | `*.tf`, `*.tofu`, `terragrunt.hcl`, `serverless.yml` | `.terraform.lock.hcl` | `terraform init` / `tofu init` |
| **Coverage & Tests** | `coverage`, `.nyc_output`, `htmlcov`, `test-results`, `playwright-report`, `.playwright` | Project files | N/A | Re-run test suite |
| **Go** | `vendor` | `go.mod` | `go.sum` | `go mod vendor` |
| **.NET / C#** | `bin`, `obj` | `*.csproj`, `*.fsproj`, `*.sln` | `packages.lock.json` | `dotnet build` |
| **C / C++** | `build`, `CMakeFiles`, `.vs` | `CMakeLists.txt`, `Makefile`, `*.sln`, `*.vcxproj` | N/A | `cmake -B build` |
| **iOS / Swift** | `.build`, `DerivedData`, `Pods` | `Package.swift`, `Podfile`, `*.xcodeproj` | `Package.resolved`, `Podfile.lock` | `swift build` / `pod install` |
| **Flutter / Dart** | `.dart_tool`, `build` | `pubspec.yaml` | `pubspec.lock` | `flutter pub get` |
| **PHP** | `vendor` | `composer.json` | `composer.lock` | `composer install` |
| **Elixir** | `_build`, `deps` | `mix.exs` | `mix.lock` | `mix deps.get` |
| **Zig** | `zig-cache`, `zig-out`, `.zig-cache` | `build.zig` | `build.zig.zon` | `zig build` |
| **Godot** | `.godot`, `.import` | `project.godot` | N/A | Re-open in Godot |
| **Unity** | `Library`, `Temp`, `Obj`, `Build`, `Builds`, `Logs`, `MemoryCaptures` | `ProjectSettings/ProjectVersion.txt` | N/A | Re-open in Unity Editor |

---

## Global Developer Caches Tab

Switch to the **Global Caches** tab with `Tab` to inspect and clean central caches:
* **Package Managers**: Cargo registry & git checkouts, npm, pnpm, Yarn, pip, uv, Bun, Pub, NuGet, Ruby gems, Coursier.
* **Build Systems & Compilers**: Go build cache, Android exploded AAR cache, Gradle cache & daemon logs, Haskell Stack indices & snapshots, Xcode DerivedData, CocoaPods, Terraform plugin cache.
* **AI & LLM Weights**: Ollama local models (`.ollama/models`), LM Studio models, HuggingFace Hub, PyTorch Hub, OpenAI Whisper weights.
* **IDE Indexes & State**: JetBrains system caches, VS Code workspace storage, Cursor workspace storage.

---

## TUI Keybindings

| Key | Action |
| :--- | :--- |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Space` | Toggle selection (on a project group, selects or deselects all child artifacts) |
| `Tab` | Switch between **Project Artifacts** and **Global Caches** tabs |
| `Enter` / `e` | Expand / collapse highlighted project group |
| `→` / `l` | Expand highlighted project group |
| `←` / `h` | Collapse highlighted project group (or jump from child to parent header) |
| `E` | Toggle Expand All / Collapse All |
| `a` | Toggle select all / deselect all |
| `s` | Cycle sort mode (`Size ↓`, `Inactivity Age ↓`, `Name A-Z`, `Ecosystem A-Z`) |
| `/` | Open search and filter input bar |
| `d` | Open deletion confirmation modal |
| `r` | Rescan directory roots and global caches |
| `q` / `Ctrl+C` | Quit |

### Search Filtering Syntax

When typing in the search bar (`/`), matches apply across project names, paths, folder names, labels, and ecosystems. You can also use filter tokens:
* `eco:ruby`, `eco:tf`, `eco:node`, `eco:python`, `eco:rust` (filter by ecosystem)
* `size:>100m`, `size:>1g`, `size:<500k` (filter by size threshold)
* `locked:yes`, `locked:no` (filter by lockfile presence)
* `git:clean`, `git:dirty` (filter by repository working directory status)

Combine tokens freely, for example: `eco:python size:>50m git:clean`.

### Deletion Modal Controls

* `t` / `T` : Move selected items to **Trash / Recycle Bin** (safest)
* `p` / `P` : **Permanently Delete** selected items
* `Esc` / `c` : Cancel and return to main view

---

## CLI Usage

### Launch Interactive TUI
```powershell
# Scan current directory
degunk

# Scan specific directories
degunk D:\projects C:\Users\YourName\dev
```

### Non-Interactive Scan
```powershell
# Print table of found artifacts
degunk --scan .

# Output formatted JSON for automation
degunk --scan . --json

# Filter by ecosystem
degunk --scan . --types ruby,scala,terraform,node

# Only show targets inactive for more than 60 days
degunk --scan . --older-than 60
```

### Automated Cleaning
```powershell
# Preview what would be cleaned without touching disk
degunk --clean-all --dry-run .

# Clean all artifacts older than 90 days, moving them to Trash
degunk --clean-all --trash --older-than 90 .

# Permanently delete all Rust target directories
degunk --clean-all --permanent --types rust .
```

---

## Development & Testing

```powershell
# Check compiler diagnostics
cargo check

# Run all unit and integration tests
cargo test
```
