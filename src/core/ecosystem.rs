use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ecosystem {
    Node,
    Python,
    Rust,
    Java,
    Go,
    DotNet,
    Cpp,
    Swift,
    Flutter,
    Php,
    Elixir,
    Zig,
    Godot,
    Unity,
    Coverage,
    Ruby,
    Scala,
    Haskell,
    Ocaml,
    Terraform,
}

impl Ecosystem {
    pub fn name(&self) -> &'static str {
        match self {
            Ecosystem::Node => "Node.js",
            Ecosystem::Python => "Python",
            Ecosystem::Rust => "Rust",
            Ecosystem::Java => "Java/Gradle/Maven",
            Ecosystem::Go => "Go",
            Ecosystem::DotNet => ".NET",
            Ecosystem::Cpp => "C/C++",
            Ecosystem::Swift => "Swift/Xcode",
            Ecosystem::Flutter => "Flutter/Dart",
            Ecosystem::Php => "PHP",
            Ecosystem::Elixir => "Elixir",
            Ecosystem::Zig => "Zig",
            Ecosystem::Godot => "Godot",
            Ecosystem::Unity => "Unity",
            Ecosystem::Coverage => "Coverage",
            Ecosystem::Ruby => "Ruby",
            Ecosystem::Scala => "Scala/sbt",
            Ecosystem::Haskell => "Haskell",
            Ecosystem::Ocaml => "OCaml",
            Ecosystem::Terraform => "Terraform/IaC",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Ecosystem::Node => "Node",
            Ecosystem::Python => "Py",
            Ecosystem::Rust => "Rust",
            Ecosystem::Java => "Java",
            Ecosystem::Go => "Go",
            Ecosystem::DotNet => ".NET",
            Ecosystem::Cpp => "C++",
            Ecosystem::Swift => "Swift",
            Ecosystem::Flutter => "Flutter",
            Ecosystem::Php => "PHP",
            Ecosystem::Elixir => "Elixir",
            Ecosystem::Zig => "Zig",
            Ecosystem::Godot => "Godot",
            Ecosystem::Unity => "Unity",
            Ecosystem::Coverage => "Cov",
            Ecosystem::Ruby => "Ruby",
            Ecosystem::Scala => "Scala",
            Ecosystem::Haskell => "Hs",
            Ecosystem::Ocaml => "OCaml",
            Ecosystem::Terraform => "IaC",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" | "js" | "ts" | "javascript" | "typescript" => Some(Ecosystem::Node),
            "python" | "py" => Some(Ecosystem::Python),
            "rust" | "rs" | "cargo" => Some(Ecosystem::Rust),
            "java" | "gradle" | "maven" | "kotlin" => Some(Ecosystem::Java),
            "go" | "golang" => Some(Ecosystem::Go),
            "dotnet" | "csharp" | "cs" | ".net" => Some(Ecosystem::DotNet),
            "c" | "cpp" | "c++" | "cmake" => Some(Ecosystem::Cpp),
            "swift" | "xcode" | "ios" => Some(Ecosystem::Swift),
            "flutter" | "dart" => Some(Ecosystem::Flutter),
            "php" | "composer" => Some(Ecosystem::Php),
            "elixir" | "mix" => Some(Ecosystem::Elixir),
            "zig" => Some(Ecosystem::Zig),
            "godot" => Some(Ecosystem::Godot),
            "unity" => Some(Ecosystem::Unity),
            "coverage" | "cov" => Some(Ecosystem::Coverage),
            "ruby" | "rb" | "gem" | "rails" => Some(Ecosystem::Ruby),
            "scala" | "sbt" => Some(Ecosystem::Scala),
            "haskell" | "hs" | "cabal" | "stack" => Some(Ecosystem::Haskell),
            "ocaml" | "ml" | "dune" | "opam" => Some(Ecosystem::Ocaml),
            "terraform" | "tf" | "tofu" | "opentofu" | "iac" | "devops" => Some(Ecosystem::Terraform),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArtifactRule {
    pub label: &'static str,
    pub ecosystem: Ecosystem,
    pub folder_names: &'static [&'static str],
    pub required_manifests: &'static [&'static str],
    pub lockfiles: &'static [&'static str],
    pub reinstall_cmd: &'static str,
}

pub static RULES: &[ArtifactRule] = &[
    // Node.js
    ArtifactRule {
        label: "Node Modules",
        ecosystem: Ecosystem::Node,
        folder_names: &["node_modules"],
        required_manifests: &["package.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm install",
    },
    ArtifactRule {
        label: "Next.js Build Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".next"],
        required_manifests: &["package.json", "next.config.js", "next.config.mjs", "next.config.ts"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Nuxt Build Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".nuxt"],
        required_manifests: &["package.json", "nuxt.config.js", "nuxt.config.ts"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npx nuxi build",
    },
    ArtifactRule {
        label: "Turbo Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".turbo"],
        required_manifests: &["package.json", "turbo.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "turbo build",
    },
    ArtifactRule {
        label: "SvelteKit Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".svelte-kit"],
        required_manifests: &["package.json", "svelte.config.js"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Angular Build Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".angular"],
        required_manifests: &["angular.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "ng build",
    },
    ArtifactRule {
        label: "Astro Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".astro"],
        required_manifests: &["astro.config.mjs", "astro.config.js", "astro.config.ts", "package.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Parcel Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".parcel-cache"],
        required_manifests: &["package.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Vite & Docs Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".vite", ".docusaurus", "storybook-static"],
        required_manifests: &["package.json", "vite.config.js", "vite.config.ts", "vite.config.mjs", "docusaurus.config.js"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Nx Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".nx"],
        required_manifests: &["nx.json", "package.json"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock"],
        reinstall_cmd: "npx nx reset",
    },
    // Python
    ArtifactRule {
        label: "Python Virtualenv",
        ecosystem: Ecosystem::Python,
        folder_names: &[".venv", "venv", "env"],
        required_manifests: &["pyproject.toml", "requirements.txt", "Pipfile", "setup.py", "setup.cfg"],
        lockfiles: &["poetry.lock", "Pipfile.lock", "pdm.lock", "uv.lock", "requirements.lock", "requirements.txt"],
        reinstall_cmd: "python -m venv .venv && pip install -r requirements.txt",
    },
    ArtifactRule {
        label: "Python Cache",
        ecosystem: Ecosystem::Python,
        folder_names: &["__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache"],
        required_manifests: &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
            "__init__.py",
            "__manifest__.py",
            "*.py",
        ],
        lockfiles: &[],
        reinstall_cmd: "Automatic upon execution",
    },
    ArtifactRule {
        label: "Python Build & Dist",
        ecosystem: Ecosystem::Python,
        folder_names: &["dist", "build", "*.egg-info"],
        required_manifests: &["pyproject.toml", "setup.py", "setup.cfg"],
        lockfiles: &[],
        reinstall_cmd: "python -m build",
    },
    ArtifactRule {
        label: "Tox / Nox Envs",
        ecosystem: Ecosystem::Python,
        folder_names: &[".tox", ".nox"],
        required_manifests: &["tox.ini", "noxfile.py", "pyproject.toml"],
        lockfiles: &[],
        reinstall_cmd: "tox / nox",
    },
    ArtifactRule {
        label: "Pixi Environment",
        ecosystem: Ecosystem::Python,
        folder_names: &[".pixi"],
        required_manifests: &["pixi.toml"],
        lockfiles: &["pixi.lock"],
        reinstall_cmd: "pixi install",
    },
    ArtifactRule {
        label: "Jupyter Checkpoints",
        ecosystem: Ecosystem::Python,
        folder_names: &[".ipynb_checkpoints"],
        required_manifests: &["*.ipynb", "pyproject.toml", "requirements.txt"],
        lockfiles: &[],
        reinstall_cmd: "Automatic upon execution",
    },
    // Rust
    ArtifactRule {
        label: "Cargo Target",
        ecosystem: Ecosystem::Rust,
        folder_names: &["target"],
        required_manifests: &["Cargo.toml"],
        lockfiles: &["Cargo.lock"],
        reinstall_cmd: "cargo build",
    },
    // Java / Gradle / Maven
    ArtifactRule {
        label: "Gradle Build",
        ecosystem: Ecosystem::Java,
        folder_names: &["build", ".gradle"],
        required_manifests: &["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts", "gradlew"],
        lockfiles: &["gradle.lockfile", "build.gradle", "build.gradle.kts"],
        reinstall_cmd: "./gradlew build",
    },
    ArtifactRule {
        label: "Maven Target",
        ecosystem: Ecosystem::Java,
        folder_names: &["target"],
        required_manifests: &["pom.xml"],
        lockfiles: &["pom.xml"],
        reinstall_cmd: "mvn clean install",
    },
    // Go
    ArtifactRule {
        label: "Go Vendor",
        ecosystem: Ecosystem::Go,
        folder_names: &["vendor"],
        required_manifests: &["go.mod"],
        lockfiles: &["go.sum"],
        reinstall_cmd: "go mod vendor",
    },
    // .NET
    ArtifactRule {
        label: ".NET Bin/Obj",
        ecosystem: Ecosystem::DotNet,
        folder_names: &["bin", "obj"],
        required_manifests: &["*.csproj", "*.fsproj", "*.sln"],
        lockfiles: &["packages.lock.json"],
        reinstall_cmd: "dotnet build",
    },
    // C / C++
    ArtifactRule {
        label: "CMake Build",
        ecosystem: Ecosystem::Cpp,
        folder_names: &["build", "CMakeFiles"],
        required_manifests: &["CMakeLists.txt", "Makefile"],
        lockfiles: &[],
        reinstall_cmd: "cmake -B build",
    },
    ArtifactRule {
        label: "Visual Studio Cache",
        ecosystem: Ecosystem::Cpp,
        folder_names: &[".vs"],
        required_manifests: &["*.sln", "*.vcxproj", "CMakeLists.txt"],
        lockfiles: &[],
        reinstall_cmd: "Open in Visual Studio",
    },
    // Swift / iOS
    ArtifactRule {
        label: "Swift Build",
        ecosystem: Ecosystem::Swift,
        folder_names: &[".build", "DerivedData", "Pods"],
        required_manifests: &["Package.swift", "Podfile"],
        lockfiles: &["Package.resolved", "Podfile.lock"],
        reinstall_cmd: "swift build / pod install",
    },
    // Flutter / Dart
    ArtifactRule {
        label: "Flutter / Dart Build",
        ecosystem: Ecosystem::Flutter,
        folder_names: &[".dart_tool", "build"],
        required_manifests: &["pubspec.yaml"],
        lockfiles: &["pubspec.lock"],
        reinstall_cmd: "flutter pub get",
    },
    // PHP
    ArtifactRule {
        label: "Composer Vendor",
        ecosystem: Ecosystem::Php,
        folder_names: &["vendor"],
        required_manifests: &["composer.json"],
        lockfiles: &["composer.lock"],
        reinstall_cmd: "composer install",
    },
    // Elixir
    ArtifactRule {
        label: "Elixir Build & Deps",
        ecosystem: Ecosystem::Elixir,
        folder_names: &["_build", "deps"],
        required_manifests: &["mix.exs"],
        lockfiles: &["mix.lock"],
        reinstall_cmd: "mix deps.get && mix compile",
    },
    // Zig
    ArtifactRule {
        label: "Zig Build Cache",
        ecosystem: Ecosystem::Zig,
        folder_names: &["zig-cache", "zig-out"],
        required_manifests: &["build.zig", "build.zig.zon"],
        lockfiles: &["build.zig.zon"],
        reinstall_cmd: "zig build",
    },
    // Godot
    ArtifactRule {
        label: "Godot Cache",
        ecosystem: Ecosystem::Godot,
        folder_names: &[".godot"],
        required_manifests: &["project.godot"],
        lockfiles: &[],
        reinstall_cmd: "Open in Godot",
    },
    // Unity
    ArtifactRule {
        label: "Unity Cache & Temp",
        ecosystem: Ecosystem::Unity,
        folder_names: &["Library", "Temp", "Obj"],
        required_manifests: &["ProjectSettings"],
        lockfiles: &[],
        reinstall_cmd: "Open in Unity",
    },
    // Ruby
    ArtifactRule {
        label: "Ruby Gems & Bundle",
        ecosystem: Ecosystem::Ruby,
        folder_names: &[".bundle", "vendor"],
        required_manifests: &["Gemfile"],
        lockfiles: &["Gemfile.lock"],
        reinstall_cmd: "bundle install",
    },
    // Scala / sbt
    ArtifactRule {
        label: "sbt Build & Bloop",
        ecosystem: Ecosystem::Scala,
        folder_names: &["target", ".bloop", ".metals"],
        required_manifests: &["build.sbt"],
        lockfiles: &["build.sbt"],
        reinstall_cmd: "sbt compile",
    },
    // Haskell
    ArtifactRule {
        label: "Haskell Build",
        ecosystem: Ecosystem::Haskell,
        folder_names: &[".stack-work", "dist-newstyle"],
        required_manifests: &["stack.yaml", "*.cabal", "cabal.project"],
        lockfiles: &["stack.yaml.lock", "cabal.project.freeze"],
        reinstall_cmd: "stack build / cabal build",
    },
    // OCaml
    ArtifactRule {
        label: "Dune Build",
        ecosystem: Ecosystem::Ocaml,
        folder_names: &["_build"],
        required_manifests: &["dune-project", "dune"],
        lockfiles: &["dune.lock"],
        reinstall_cmd: "dune build",
    },
    // Terraform / DevOps
    ArtifactRule {
        label: "Terraform Providers Cache",
        ecosystem: Ecosystem::Terraform,
        folder_names: &[".terraform"],
        required_manifests: &["*.tf", "*.tofu", "terragrunt.hcl"],
        lockfiles: &[".terraform.lock.hcl"],
        reinstall_cmd: "terraform init",
    },
    ArtifactRule {
        label: "Serverless & SAM Artifacts",
        ecosystem: Ecosystem::Terraform,
        folder_names: &[".serverless", ".aws-sam"],
        required_manifests: &["serverless.yml", "serverless.ts", "template.yaml", "samconfig.toml"],
        lockfiles: &[],
        reinstall_cmd: "serverless package / sam build",
    },
    // Coverage & Test Output
    ArtifactRule {
        label: "Test Coverage & Reports",
        ecosystem: Ecosystem::Coverage,
        folder_names: &["coverage", ".nyc_output", "htmlcov", "test-results", "playwright-report", ".playwright"],
        required_manifests: &[
            "package.json",
            "pyproject.toml",
            "Cargo.toml",
            "go.mod",
            "playwright.config.ts",
            "playwright.config.js",
        ],
        lockfiles: &[],
        reinstall_cmd: "Run test suite",
    },
];

/// Checks whether a directory name matches any artifact rule.
pub fn match_rule(folder_name: &str, parent_path: &Path) -> Option<&'static ArtifactRule> {
    for rule in RULES {
        if rule.folder_names.iter().any(|&f| {
            if f.starts_with("*.") {
                let ext = &f[1..];
                folder_name.to_lowercase().ends_with(&ext.to_lowercase())
            } else {
                f.eq_ignore_ascii_case(folder_name)
            }
        }) {
            // Verify at least one manifest matches in the parent directory
            if has_manifest(parent_path, rule.required_manifests) {
                return Some(rule);
            }
            // Self-identifying Python virtualenvs: if the folder contains pyvenv.cfg, it's a virtualenv
            if rule.ecosystem == Ecosystem::Python
                && parent_path.join(folder_name).join("pyvenv.cfg").exists()
            {
                return Some(rule);
            }
        }
    }
    None
}

/// Checks if any required manifest exists in the directory.
pub fn has_manifest(dir: &Path, manifests: &[&'static str]) -> bool {
    manifests.iter().any(|&m| {
        if m.starts_with("*.") {
            // Wildcard extension check
            let ext = &m[2..];
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(e) = entry.path().extension() {
                        if e.to_string_lossy().eq_ignore_ascii_case(ext) {
                            return true;
                        }
                    }
                }
            }
            false
        } else {
            dir.join(m).exists()
        }
    })
}

/// Checks if any known lockfile exists in the directory.
pub fn has_lockfile(dir: &Path, lockfiles: &[&'static str]) -> (bool, Option<String>) {
    if lockfiles.is_empty() {
        return (true, None); // Not applicable
    }
    for &lock in lockfiles {
        if dir.join(lock).exists() {
            return (true, Some(lock.to_string()));
        }
    }
    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_ecosystem_parse() {
        assert_eq!(Ecosystem::parse("node"), Some(Ecosystem::Node));
        assert_eq!(Ecosystem::parse("python"), Some(Ecosystem::Python));
        assert_eq!(Ecosystem::parse("rust"), Some(Ecosystem::Rust));
        assert_eq!(Ecosystem::parse("golang"), Some(Ecosystem::Go));
        assert_eq!(Ecosystem::parse("ruby"), Some(Ecosystem::Ruby));
        assert_eq!(Ecosystem::parse("scala"), Some(Ecosystem::Scala));
        assert_eq!(Ecosystem::parse("haskell"), Some(Ecosystem::Haskell));
        assert_eq!(Ecosystem::parse("ocaml"), Some(Ecosystem::Ocaml));
        assert_eq!(Ecosystem::parse("terraform"), Some(Ecosystem::Terraform));
        assert_eq!(Ecosystem::parse("unknown_xyz"), None);
    }

    #[test]
    fn test_match_rule_node() {
        let temp_dir = std::env::temp_dir().join("degunk_test_match_node");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = File::create(temp_dir.join("package.json"));

        let matched = match_rule("node_modules", &temp_dir);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().ecosystem, Ecosystem::Node);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
