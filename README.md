# degunk

Developers lose tens of gigabytes of disk space to old `node_modules`, `venv`, `target`, and build caches in projects they haven't touched in months. `degunk` finds those discarded folders, shows you how long it's been since you committed to each repo, and lets you reclaim disk space safely.

---

## Features

* **Interactive path launcher**: Running `degunk` opens a path picker with your common dev folders (`~/Projects`, `~/dev`), drives, and a custom path input with paste support.
* **Project tree view**: Monorepos, mobile apps, and fullstack projects group multiple build targets under their project root. You can collapse and expand groups with `Enter` or `e`.
* **Deep ecosystem coverage**: Scans 20+ stacks including Node.js, Python, Rust, Go, Java/Kotlin, .NET, C/C++, Swift/iOS, Flutter, Zig, Godot, Unity, Ruby, Scala, Haskell, OCaml, and Terraform.
* **Global developer caches**: Press `Tab` to see central caches like Cargo package checkouts, npm, pnpm, pip, Gradle daemons, Go build caches, and local AI model weights (Ollama, LM Studio, HuggingFace).
* **Git activity context**: Inspects each project's Git history to display days since your last commit, uncommitted local changes, and unpushed commits before you delete anything.
* **Search filters**: Filter by ecosystem, size, or safety state using tokens like `eco:node`, `size:>500m`, `git:clean`, or `locked:yes`.
* **Safe deletion**: Moves files to your OS Trash or Recycle Bin by default, with an option for direct deletion. Handles read-only file locks on Windows and Unix cleanly.

---

## Quick start

### Launch the TUI

```bash
# Open the interactive path picker
degunk

# Scan the current directory immediately
degunk .

# Scan specific project folders or drives
degunk D:\projects ~/code
```

### CLI and scripting

```bash
# Print a summary table without launching the full TUI
degunk --scan .

# Output machine-readable JSON
degunk --scan . --json

# Filter by ecosystem
degunk --scan . --types node,rust,python

# Only show targets untouched for over 60 days
degunk --scan . --older-than 60

# Clean old artifacts into the Trash without manual confirmation
degunk --clean-all --trash --older-than 90 .
```

---

## Keyboard shortcuts

| Key | Action |
| :--- | :--- |
| `↑` / `k`, `↓` / `j` | Move selection up or down |
| `Space` | Select or deselect item (selecting a project selects all its build targets) |
| `Enter` / `e` | Expand or collapse project folder |
| `E` | Expand all or collapse all project groups |
| `Tab` | Switch between **Projects** and **Global Caches** views |
| `s` | Cycle sort order (Size, Inactivity age, Name, Ecosystem) |
| `/` | Search projects and filter by token |
| `p` | Open the path picker to scan a different drive or directory |
| `d` | Open deletion confirmation modal |
| `r` | Rescan current paths and global caches |
| `q` / `Ctrl+C` | Quit |

### Search syntax

Type `/` to search names, folder names, and paths. You can also mix in filter tokens:

* `eco:rust`, `eco:node`, `eco:python`
* `size:>100m`, `size:>1g`, `size:<50m`
* `locked:yes`, `locked:no`
* `git:clean`, `git:dirty`

Example: `/eco:python size:>100m git:clean`

---

## Supported ecosystems and targets

| Ecosystem | Target folders | Key markers | Lockfile |
| :--- | :--- | :--- | :--- |
| **Node.js / Web** | `node_modules`, `.next`, `.nuxt`, `.turbo`, `.svelte-kit`, `.angular`, `.astro`, `.parcel-cache`, `.vite`, `.docusaurus`, `storybook-static` | `package.json`, `next.config.*`, etc. | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` |
| **Python** | `.venv`, `venv`, `env`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `dist`, `build`, `*.egg-info`, `.tox`, `.nox`, `.pixi` | `pyproject.toml`, `requirements.txt`, `Pipfile`, `setup.py` | `poetry.lock`, `Pipfile.lock`, `uv.lock`, `pdm.lock` |
| **Rust** | `target` | `Cargo.toml` | `Cargo.lock` |
| **Java / Kotlin** | `build`, `.gradle`, `target` | `build.gradle`, `build.gradle.kts`, `pom.xml` | `gradle.lockfile` |
| **Go** | `vendor` | `go.mod` | `go.sum` |
| **.NET / C#** | `bin`, `obj` | `*.csproj`, `*.fsproj`, `*.sln` | `packages.lock.json` |
| **C / C++** | `build`, `CMakeFiles`, `.vs` | `CMakeLists.txt`, `Makefile`, `*.sln` | N/A |
| **iOS / Swift** | `.build`, `DerivedData`, `Pods` | `Package.swift`, `Podfile`, `*.xcodeproj` | `Package.resolved`, `Podfile.lock` |
| **Flutter / Dart** | `.dart_tool`, `build` | `pubspec.yaml` | `pubspec.lock` |
| **PHP** | `vendor` | `composer.json` | `composer.lock` |
| **Elixir** | `_build`, `deps` | `mix.exs` | `mix.lock` |
| **Zig** | `zig-cache`, `zig-out`, `.zig-cache` | `build.zig` | `build.zig.zon` |
| **Godot** | `.godot`, `.import` | `project.godot` | N/A |
| **Unity** | `Library`, `Temp`, `Obj`, `Build`, `Builds`, `Logs` | `ProjectSettings/ProjectVersion.txt` | N/A |
| **Ruby** | `.bundle`, `vendor` | `Gemfile` | `Gemfile.lock` |
| **Scala** | `target`, `.bloop`, `.metals` | `build.sbt` | N/A |
| **Haskell** | `.stack-work`, `dist-newstyle` | `stack.yaml`, `*.cabal` | N/A |
| **OCaml** | `_build` | `dune-project`, `dune` | `dune.lock` |
| **Terraform / IaC** | `.terraform`, `.serverless`, `.aws-sam` | `*.tf`, `*.tofu`, `serverless.yml` | `.terraform.lock.hcl` |
| **Coverage & Tests** | `coverage`, `.nyc_output`, `htmlcov`, `test-results`, `playwright-report` | Project test runners | N/A |

---

## Global tool caches

Press `Tab` in the TUI to inspect central tool caches that live outside your project folders:

* **Package managers**: Cargo checkouts and git registries, npm, pnpm, Yarn, pip, uv, Bun, Pub, NuGet, Ruby gems, Coursier.
* **Compilers and build systems**: Go build cache, Gradle daemon logs and caches, Android build artifacts, Xcode DerivedData, CocoaPods, Terraform plugins.
* **AI model weights**: Ollama models (`~/.ollama/models`), LM Studio, HuggingFace Hub, PyTorch Hub cache, Whisper weights.
* **Editor caches**: JetBrains system caches, VS Code workspace storage, Cursor workspace storage.

---

## Development

```bash
# Check compiler diagnostics
cargo check

# Run test suite
cargo test
```
