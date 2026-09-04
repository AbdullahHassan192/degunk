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
    Unreal,
    R,
    Android,
    ReactNative,
    Embedded,
    Elm,
    Clojure,
    Julia,
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
            Ecosystem::Unreal => "Unreal Engine",
            Ecosystem::R => "R",
            Ecosystem::Android => "Android/NDK",
            Ecosystem::ReactNative => "React Native/Expo",
            Ecosystem::Embedded => "Embedded/PlatformIO",
            Ecosystem::Elm => "Elm",
            Ecosystem::Clojure => "Clojure",
            Ecosystem::Julia => "Julia",
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
            Ecosystem::Unreal => "Unreal",
            Ecosystem::R => "R",
            Ecosystem::Android => "Droid",
            Ecosystem::ReactNative => "RN",
            Ecosystem::Embedded => "Emb",
            Ecosystem::Elm => "Elm",
            Ecosystem::Clojure => "Clj",
            Ecosystem::Julia => "Julia",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" | "js" | "ts" | "javascript" | "typescript" | "web" => {
                Some(Ecosystem::Node)
            }
            "python" | "py" => Some(Ecosystem::Python),
            "rust" | "rs" | "cargo" => Some(Ecosystem::Rust),
            "java" | "gradle" | "maven" | "kotlin" => Some(Ecosystem::Java),
            "go" | "golang" => Some(Ecosystem::Go),
            "dotnet" | "csharp" | "cs" | ".net" | "fsharp" | "fs" => Some(Ecosystem::DotNet),
            "c" | "cpp" | "c++" | "cmake" => Some(Ecosystem::Cpp),
            "swift" | "xcode" | "ios" | "carthage" => Some(Ecosystem::Swift),
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
            "terraform" | "tf" | "tofu" | "opentofu" | "iac" | "devops" => {
                Some(Ecosystem::Terraform)
            }
            "unreal" | "ue4" | "ue5" | "epic" => Some(Ecosystem::Unreal),
            "r" | "rstats" | "renv" | "rstudio" => Some(Ecosystem::R),
            "android" | "ndk" | "aar" => Some(Ecosystem::Android),
            "react-native" | "reactnative" | "rn" | "expo" => Some(Ecosystem::ReactNative),
            "embedded" | "emb" | "pio" | "platformio" | "arduino" => Some(Ecosystem::Embedded),
            "elm" => Some(Ecosystem::Elm),
            "clojure" | "clj" | "cljs" | "lein" | "leiningen" => Some(Ecosystem::Clojure),
            "julia" | "jl" => Some(Ecosystem::Julia),
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
    ArtifactRule {
        label: "Modern Web & Server Output",
        ecosystem: Ecosystem::Node,
        folder_names: &[".output", ".swc", ".nitro", ".temp"],
        required_manifests: &["package.json", "nuxt.config.js", "nuxt.config.ts", "nitro.config.ts"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
    },
    ArtifactRule {
        label: "Cloudflare & Serverless Output",
        ecosystem: Ecosystem::Node,
        folder_names: &[".wrangler", ".vercel", ".netlify", ".sst"],
        required_manifests: &[
            "wrangler.toml",
            "wrangler.json",
            "wrangler.jsonc",
            "vercel.json",
            "netlify.toml",
            "sst.config.ts",
            "package.json",
        ],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build / wrangler deploy",
    },
    ArtifactRule {
        label: "Web Bundler & Lint Cache",
        ecosystem: Ecosystem::Node,
        folder_names: &[".cache"],
        required_manifests: &[
            "package.json",
            "gatsby-config.js",
            "gatsby-config.ts",
            "gatsby-config.mjs",
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.js",
        ],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npm run build",
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
    ArtifactRule {
        label: ".NET Test & Benchmark Artifacts",
        ecosystem: Ecosystem::DotNet,
        folder_names: &["BenchmarkDotNet.Artifacts", "TestResults"],
        required_manifests: &["*.csproj", "*.fsproj", "*.sln"],
        lockfiles: &["packages.lock.json"],
        reinstall_cmd: "dotnet test / dotnet run -c Release",
    },
    // C / C++
    ArtifactRule {
        label: "CMake Build",
        ecosystem: Ecosystem::Cpp,
        folder_names: &[
            "build",
            "CMakeFiles",
            "cmake-build-debug",
            "cmake-build-release",
            "cmake-build-relwithdebinfo",
            "cmake-build-minsizerel",
            "builddir",
        ],
        required_manifests: &["CMakeLists.txt", "Makefile", "meson.build"],
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
    ArtifactRule {
        label: "vcpkg Installed Packages",
        ecosystem: Ecosystem::Cpp,
        folder_names: &["vcpkg_installed"],
        required_manifests: &["vcpkg.json", "vcpkg-configuration.json", "CMakeLists.txt"],
        lockfiles: &["vcpkg-lock.json", "vcpkg.json"],
        reinstall_cmd: "vcpkg install",
    },
    // Swift / iOS
    ArtifactRule {
        label: "Swift Build",
        ecosystem: Ecosystem::Swift,
        folder_names: &[".build", "DerivedData", "Pods", "Carthage"],
        required_manifests: &[
            "Package.swift",
            "Podfile",
            "Cartfile",
            "Cartfile.resolved",
            "*.xcodeproj",
            "*.xcworkspace",
        ],
        lockfiles: &["Package.resolved", "Podfile.lock", "Cartfile.resolved"],
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
    // Unreal Engine
    ArtifactRule {
        label: "Unreal Engine Build & Cache",
        ecosystem: Ecosystem::Unreal,
        folder_names: &["Intermediate", "Saved", "DerivedDataCache", "Binaries"],
        required_manifests: &["*.uproject", "*.uplugin"],
        lockfiles: &[],
        reinstall_cmd: "Regenerate project files & build in Unreal / Visual Studio",
    },
    // R
    ArtifactRule {
        label: "R Project & renv Cache",
        ecosystem: Ecosystem::R,
        folder_names: &[".Rproj.user", ".Rcache", "library"],
        required_manifests: &["*.Rproj", "renv.lock", "DESCRIPTION"],
        lockfiles: &["renv.lock"],
        reinstall_cmd: "R -e 'renv::restore()'",
    },
    // Android / NDK
    ArtifactRule {
        label: "Android NDK & C++ Build",
        ecosystem: Ecosystem::Android,
        folder_names: &[".cxx", ".externalNativeBuild"],
        required_manifests: &[
            "build.gradle",
            "build.gradle.kts",
            "CMakeLists.txt",
            "settings.gradle",
            "settings.gradle.kts",
        ],
        lockfiles: &["gradle.lockfile"],
        reinstall_cmd: "./gradlew assembleDebug",
    },
    // React Native / Expo
    ArtifactRule {
        label: "Expo & React Native Cache",
        ecosystem: Ecosystem::ReactNative,
        folder_names: &[".expo", ".metro-health-check"],
        required_manifests: &["app.json", "package.json", "metro.config.js", "expo-env.d.ts"],
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "bun.lock"],
        reinstall_cmd: "npx expo start / npx react-native start",
    },
    // Embedded / PlatformIO
    ArtifactRule {
        label: "PlatformIO Build & Libs",
        ecosystem: Ecosystem::Embedded,
        folder_names: &[".pio"],
        required_manifests: &["platformio.ini"],
        lockfiles: &[],
        reinstall_cmd: "pio run",
    },
    // Elm
    ArtifactRule {
        label: "Elm Build Cache",
        ecosystem: Ecosystem::Elm,
        folder_names: &["elm-stuff"],
        required_manifests: &["elm.json"],
        lockfiles: &[],
        reinstall_cmd: "elm make",
    },
    // Clojure
    ArtifactRule {
        label: "Clojure Build Cache",
        ecosystem: Ecosystem::Clojure,
        folder_names: &[".cpcache", ".shadow-cljs", ".calva"],
        required_manifests: &["project.clj", "deps.edn", "shadow-cljs.edn"],
        lockfiles: &[],
        reinstall_cmd: "clj -M / lein compile",
    },
    // Julia
    ArtifactRule {
        label: "Julia Cache",
        ecosystem: Ecosystem::Julia,
        folder_names: &[".julia"],
        required_manifests: &["Project.toml", "JuliaProject.toml"],
        lockfiles: &["Manifest.toml"],
        reinstall_cmd: "julia --project -e 'using Pkg; Pkg.instantiate()'",
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
            // 1. Verify at least one manifest matches in the parent directory
            if has_manifest(parent_path, rule.required_manifests) {
                return Some(rule);
            }

            // 2. Check parent's parent for nested conventions (e.g. renv/library, ios/build, android/app/build)
            if let Some(grandparent) = parent_path.parent() {
                if has_manifest(grandparent, rule.required_manifests) {
                    let parent_name = parent_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if (rule.ecosystem == Ecosystem::R && parent_name.eq_ignore_ascii_case("renv"))
                        || ((rule.ecosystem == Ecosystem::ReactNative
                            || rule.ecosystem == Ecosystem::Node
                            || rule.ecosystem == Ecosystem::Swift)
                            && (parent_name.eq_ignore_ascii_case("ios")
                                || parent_name.eq_ignore_ascii_case("android")
                                || parent_name.eq_ignore_ascii_case("app")))
                    {
                        return Some(rule);
                    }
                }
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
    // 1. Fast path: check exact filenames directly with cheap stat checks
    for &m in manifests {
        if !m.starts_with("*.") && dir.join(m).exists() {
            return true;
        }
    }

    // 2. Identify wildcard patterns
    let wildcards: Vec<&'static str> = manifests
        .iter()
        .copied()
        .filter(|m| m.starts_with("*."))
        .collect();

    if wildcards.is_empty() {
        return false;
    }

    // 3. Read directory entries once and test against all wildcard patterns
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            for &w in &wildcards {
                let suffix = &w[1..]; // e.g. ".sln", ".vcxproj", ".uproject", ".Rproj"
                if name_str.len() >= suffix.len()
                    && name_str[name_str.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
                {
                    return true;
                }
            }
        }
    }

    false
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
        assert_eq!(Ecosystem::parse("unreal"), Some(Ecosystem::Unreal));
        assert_eq!(Ecosystem::parse("ue5"), Some(Ecosystem::Unreal));
        assert_eq!(Ecosystem::parse("r"), Some(Ecosystem::R));
        assert_eq!(Ecosystem::parse("android"), Some(Ecosystem::Android));
        assert_eq!(Ecosystem::parse("expo"), Some(Ecosystem::ReactNative));
        assert_eq!(Ecosystem::parse("react-native"), Some(Ecosystem::ReactNative));
        assert_eq!(Ecosystem::parse("platformio"), Some(Ecosystem::Embedded));
        assert_eq!(Ecosystem::parse("elm"), Some(Ecosystem::Elm));
        assert_eq!(Ecosystem::parse("clojure"), Some(Ecosystem::Clojure));
        assert_eq!(Ecosystem::parse("julia"), Some(Ecosystem::Julia));
        assert_eq!(Ecosystem::parse("unknown_xyz"), None);
    }

    #[test]
    fn test_match_rule_node() {
        let temp_dir = std::env::temp_dir().join("degunk_test_match_node");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = File::create(temp_dir.join("package.json"));

        let matched = match_rule("node_modules", &temp_dir);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().ecosystem, Ecosystem::Node);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_match_rule_unreal_and_r() {
        let temp_dir = std::env::temp_dir().join("degunk_test_match_unreal_r");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Unreal
        let unreal_proj = temp_dir.join("MyGame");
        std::fs::create_dir_all(&unreal_proj).unwrap();
        File::create(unreal_proj.join("MyGame.uproject")).unwrap();

        let intermediate = match_rule("Intermediate", &unreal_proj);
        assert!(intermediate.is_some());
        assert_eq!(intermediate.unwrap().ecosystem, Ecosystem::Unreal);

        let ddc = match_rule("DerivedDataCache", &unreal_proj);
        assert!(ddc.is_some());
        assert_eq!(ddc.unwrap().ecosystem, Ecosystem::Unreal);

        // R with renv/library
        let r_proj = temp_dir.join("r_analysis");
        let renv_dir = r_proj.join("renv");
        std::fs::create_dir_all(&renv_dir).unwrap();
        File::create(r_proj.join("renv.lock")).unwrap();

        let renv_lib = match_rule("library", &renv_dir);
        assert!(renv_lib.is_some());
        assert_eq!(renv_lib.unwrap().ecosystem, Ecosystem::R);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_has_manifest_wildcards_and_exact() {
        let temp_dir = std::env::temp_dir().join("degunk_test_manifest_wildcards");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let manifests = &["CMakeLists.txt", "Makefile", "*.sln", "*.vcxproj", "*.vcxproj.filters"];

        // Initially no files exist
        assert!(!has_manifest(&temp_dir, manifests));

        // Exact match
        let cmake = temp_dir.join("CMakeLists.txt");
        File::create(&cmake).unwrap();
        assert!(has_manifest(&temp_dir, manifests));
        std::fs::remove_file(&cmake).unwrap();

        // Single wildcard extension match
        let sln = temp_dir.join("MyProject.sln");
        File::create(&sln).unwrap();
        assert!(has_manifest(&temp_dir, manifests));
        std::fs::remove_file(&sln).unwrap();

        // Compound wildcard extension match
        let filters = temp_dir.join("MyProject.vcxproj.filters");
        File::create(&filters).unwrap();
        assert!(has_manifest(&temp_dir, manifests));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
