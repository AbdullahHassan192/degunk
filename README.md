# ✦ Black Hole

> A fast, cross-platform dependency and build artifact cleaner built with Rust and Ratatui.

Developers routinely lose tens of gigabytes of disk space to forgotten `node_modules`, Python `.venv`, Rust `target/`, and compiler caches across old clone directories and dormant projects. **Black Hole** scans workspaces recursively, calculates recoverable disk space in parallel, evaluates project inactivity and Git safety, and lets you wipe or trash artifacts with confidence.

---

## Features

* **Instant Multi-Ecosystem Detection**: Out-of-the-box support for 10+ ecosystems (Node.js, Python, Rust, Java/Gradle/Maven, Go, C/C++, .NET, iOS/Swift, Flutter/Dart, PHP, Elixir).
* **Git & Activity Awareness**:
  * Calculates project inactivity age using Git commit timestamps or filesystem metadata.
  * Warns before deleting projects with uncommitted Git modifications or unpushed commits.
  * Verifies lockfile existence (`package-lock.json`, `Cargo.lock`, `poetry.lock`, etc.) so you know dependencies can be reinstalled deterministically.
* **Live Streaming Scanner**: Terminal UI opens instantly and renders targets as they are discovered in the background with concurrent size calculations using Rayon.
* **Safe Deletion Choices**: Clean prompt offers both **Move to Trash** (OS Recycle Bin via `trash` crate) and **Permanent Delete** (with fast folder renaming on Windows to avoid NTFS locks).
* **Dual Interface**:
  * **Interactive TUI**: Built with Ratatui and Crossterm, featuring live search (`/`), multi-column sorting (`s`), bulk toggle (`a`), and clean confirmation dialogs (`d`).
  * **Non-Interactive CLI**: Scriptable with `--scan`, `--json`, `--types`, `--older-than`, and `--dry-run`.

---

## Codebase Architecture

```
blackhole/
├── Cargo.toml                # Dependencies and release build profiles
├── README.md                 # Project documentation
├── src/
│   ├── main.rs               # Binary entrypoint; parses CLI arguments and launches TUI or CLI runner
│   ├── lib.rs                # Library root exposing core, cli, and ui modules
│   ├── cli.rs                # Clap CLI schema and non-interactive output handler (text & JSON)
│   ├── core/
│   │   ├── mod.rs            # Core module exports
│   │   ├── ecosystem.rs      # Rule definitions for 10+ stacks (folder patterns, manifests, lockfiles)
│   │   ├── scanner.rs        # Recursive walker with pruning, live streaming, and job synchronization
│   │   ├── size.rs           # Fast concurrent disk size & file count calculator and byte formatter
│   │   ├── git.rs            # Git repository inspection (commit dates, dirty tree, unpushed commits)
│   │   └── deleter.rs        # Trash (Recycle Bin) and fast recursive permanent deletion
│   └── ui/
│       ├── mod.rs            # UI module exports and run_tui entrypoint
│       ├── app.rs            # App state machine (artifacts list, sorting, search, selection, modal)
│       ├── theme.rs          # Color palettes, ecosystem badges, and size threshold styling
│       ├── tui.rs            # Crossterm terminal setup, teardown, panic hooks, and event loop
│       └── views/
│           ├── mod.rs        # View components
│           ├── header.rs     # Header stats (scan spinner, total bytes, reclaimable selected bytes)
│           ├── table.rs      # Main artifact table with colored status badges
│           ├── search.rs     # Interactive fuzzy search bar
│           ├── modal.rs      # Deletion confirmation modal with safety warnings
│           └── footer.rs     # Keyboard navigation reference bar
└── tests/
    ├── scanner_test.rs       # Tests for node_modules and target directory detection
    └── multi_ecosystem_test.rs # Tests for multi-stack scanning and ecosystem filtering
```

---

## How the Scanning Engine Works

```
1. Recursively walk directory tree
   │
   ├── Skip system folders (AppData, Windows, Program Files, $Recycle.Bin, .git)
   │
2. Directory match found? (e.g. "node_modules", "target", ".venv")
   ├── NO  ──> Recurse into child directories
   └── YES ──> Verify parent manifest (e.g. package.json, Cargo.toml, pyproject.toml)
               │
               ├── Valid Project?
               │   ├── NO  ──> Skip (avoid deleting non-project custom folders)
               │   └── YES ──>
               │       ├── 1. Send ScanMessage::Found immediately (UI populates with zero delay)
               │       ├── 2. Spawn background worker for calculate_dir_size()
               │       ├── 3. Spawn background worker for inspect_project_activity()
               │       └── 4. PRUNE recursion: do NOT walk inside artifact subdirectories
```

---

## Supported Ecosystems & Targets

| Ecosystem | Target Folders | Required Manifests | Lockfiles | Reinstall Command |
| :--- | :--- | :--- | :--- | :--- |
| **Node.js / Web** | `node_modules`, `.next`, `.nuxt`, `.turbo`, `.svelte-kit` | `package.json`, `next.config.js`, `nuxt.config.js`, `turbo.json`, `svelte.config.js` | `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` | `npm install` / `pnpm install` |
| **Python** | `.venv`, `venv`, `env`, `__pycache__`, `.pytest_cache`, `.mypy_cache` | `pyproject.toml`, `requirements.txt`, `Pipfile`, `setup.py` | `poetry.lock`, `Pipfile.lock`, `pdm.lock`, `uv.lock` | `pip install -r requirements.txt` |
| **Rust** | `target` | `Cargo.toml` | `Cargo.lock` | `cargo build` |
| **Java / Kotlin** | `build`, `.gradle`, `target` | `build.gradle`, `build.gradle.kts`, `pom.xml`, `gradlew` | `gradle.lockfile` | `./gradlew build` / `mvn clean install` |
| **Go** | `vendor` | `go.mod` | `go.sum` | `go mod vendor` |
| **.NET / C#** | `bin`, `obj` | `*.csproj`, `*.fsproj`, `*.sln` | `packages.lock.json` | `dotnet build` |
| **C / C++** | `build`, `CMakeFiles`, `.vs` | `CMakeLists.txt`, `Makefile`, `*.sln`, `*.vcxproj` | N/A | `cmake -B build` |
| **iOS / Swift** | `.build`, `DerivedData`, `Pods` | `Package.swift`, `Podfile`, `*.xcodeproj` | `Package.resolved`, `Podfile.lock` | `swift build` / `pod install` |
| **Flutter / Dart** | `.dart_tool`, `build` | `pubspec.yaml` | `pubspec.lock` | `flutter pub get` |
| **PHP** | `vendor` | `composer.json` | `composer.lock` | `composer install` |
| **Elixir** | `_build`, `deps` | `mix.exs` | `mix.lock` | `mix deps.get` |

---

## TUI Keybindings

| Key | Action |
| :--- | :--- |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Space` | Toggle selection checkmark on highlighted item |
| `a` | Toggle select all / deselect all |
| `s` | Cycle sort mode (`Size ↓`, `Inactivity Age ↓`, `Name A-Z`, `Ecosystem A-Z`) |
| `/` | Open search and filter input bar |
| `d` | Open deletion confirmation modal (automatically selects current row if none selected) |
| `r` | Rescan directory roots |
| `q` / `Ctrl+C` | Quit |

### Deletion Modal Controls
* `t` / `T` : Move selected items to **Trash / Recycle Bin**
* `p` / `P` : **Permanently Delete** selected items
* `Esc` / `c` : Cancel and return to main table

---

## CLI Usage & Examples

### Launch Interactive TUI
```powershell
# Scan current directory
blackhole

# Scan custom directories
blackhole D:\projects C:\Users\YourName\dev
```

### Non-Interactive Scan
```powershell
# Print table of found artifacts
blackhole --scan .

# Output formatted JSON (for scripts and automation)
blackhole --scan . --json

# Filter by ecosystem
blackhole --scan . --types node,python,rust

# Only show targets inactive for more than 60 days
blackhole --scan . --older-than 60
```

### Automated Cleaning
```powershell
# Preview what would be cleaned without touching files
blackhole --clean-all --dry-run .

# Clean all artifacts older than 90 days, moving them to Trash
blackhole --clean-all --trash --older-than 90 .

# Permanently delete all Rust target directories
blackhole --clean-all --permanent --types rust .
```

---

## Development & Testing

```powershell
# Check compiler diagnostics
cargo check

# Run all unit and integration tests
cargo test

# Build optimized release binary
cargo build --release
```
The compiled release executable will be available at `target/release/blackhole.exe`.
