//! Developer Framework & Runtime Detection Engine
//!
//! Phase 2: Analyses process `cmd`, `exe`, `cwd`, and listening ports to classify
//! running processes as specific web frameworks/runtimes (Next.js, Vite, Wrangler, etc.).
//!
//! Detection strategy uses four tiers (fast-to-deep):
//! 1. CLI token matching — cheap, always runs
//! 2. Project manifest probing — reads `package.json`, `Cargo.toml`, etc. with LRU cache
//! 3. Port correlation — marks processes as "active dev server" if they hold a well-known dev port
//! 4. (Future) Process tree hierarchy propagation

use ratatui::style::Color;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Public enums
// ---------------------------------------------------------------------------

/// JavaScript / systems runtime that executes the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeType {
    Node,
    Bun,
    Deno,
    Rust,
    Python,
    Go,
    Unknown,
}

impl RuntimeType {
    /// Short human-readable label shown in UI badges.
    pub fn label(self) -> &'static str {
        match self {
            RuntimeType::Node => "Node",
            RuntimeType::Bun => "Bun",
            RuntimeType::Deno => "Deno",
            RuntimeType::Rust => "Rust",
            RuntimeType::Python => "Python",
            RuntimeType::Go => "Go",
            RuntimeType::Unknown => "",
        }
    }

    /// Accent colour for the runtime label.
    pub fn color(self) -> Color {
        match self {
            RuntimeType::Node => Color::Rgb(104, 160, 99), // node green
            RuntimeType::Bun => Color::Rgb(251, 191, 36),  // bun yellow
            RuntimeType::Deno => Color::Rgb(39, 200, 161), // deno teal
            RuntimeType::Rust => Color::Rgb(222, 95, 27),  // rust orange
            RuntimeType::Python => Color::Rgb(55, 118, 171), // python blue
            RuntimeType::Go => Color::Rgb(0, 173, 216),    // go cyan
            RuntimeType::Unknown => Color::DarkGray,
        }
    }
}

/// Framework or toolchain used by the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameworkType {
    NextJs,
    Vite,
    Astro,
    Nuxt,
    Remix,
    SvelteKit,
    Wrangler,
    Turborepo,
    Webpack,
    Esbuild,
    Parcel,
    Strapi,
    NestJs,
    Express,
    Hono,
    Fastify,
    Elysia,
    Laravel,
    Rails,
    Phoenix,
    FastAPI,
    Flask,
    Django,
    CargoWatch,
    Postgres,
    Redis,
    Mysql,
    Sqlite,
    None,
}

impl FrameworkType {
    /// Short label shown in the TYPE column.
    pub fn label(self) -> &'static str {
        match self {
            FrameworkType::NextJs => "Next.js",
            FrameworkType::Vite => "Vite",
            FrameworkType::Astro => "Astro",
            FrameworkType::Nuxt => "Nuxt",
            FrameworkType::Remix => "Remix",
            FrameworkType::SvelteKit => "SvelteKit",
            FrameworkType::Wrangler => "Wrangler",
            FrameworkType::Turborepo => "Turbo",
            FrameworkType::Webpack => "Webpack",
            FrameworkType::Esbuild => "esbuild",
            FrameworkType::Parcel => "Parcel",
            FrameworkType::Strapi => "Strapi",
            FrameworkType::NestJs => "NestJS",
            FrameworkType::Express => "Express",
            FrameworkType::Hono => "Hono",
            FrameworkType::Fastify => "Fastify",
            FrameworkType::Elysia => "Elysia",
            FrameworkType::Laravel => "Laravel",
            FrameworkType::Rails => "Rails",
            FrameworkType::Phoenix => "Phoenix",
            FrameworkType::FastAPI => "FastAPI",
            FrameworkType::Flask => "Flask",
            FrameworkType::Django => "Django",
            FrameworkType::CargoWatch => "cargo",
            FrameworkType::Postgres => "Postgres",
            FrameworkType::Redis => "Redis",
            FrameworkType::Mysql => "MySQL",
            FrameworkType::Sqlite => "SQLite",
            FrameworkType::None => "",
        }
    }

    /// Accent colour for the framework badge.
    pub fn color(self) -> Color {
        match self {
            FrameworkType::NextJs => Color::Rgb(180, 180, 180), // next = near-white
            FrameworkType::Vite => Color::Rgb(189, 52, 254),    // vite purple
            FrameworkType::Astro => Color::Rgb(255, 93, 1),     // astro orange
            FrameworkType::Nuxt => Color::Rgb(0, 220, 130),     // nuxt green
            FrameworkType::Remix => Color::Rgb(0, 171, 255),    // remix blue
            FrameworkType::SvelteKit => Color::Rgb(255, 62, 0), // svelte red
            FrameworkType::Wrangler => Color::Rgb(247, 133, 42), // cloudflare orange
            FrameworkType::Turborepo => Color::Rgb(0, 148, 255), // turbo blue
            FrameworkType::Webpack => Color::Rgb(141, 214, 249),
            FrameworkType::Esbuild => Color::Rgb(255, 204, 0),
            FrameworkType::Parcel => Color::Rgb(255, 145, 0),
            FrameworkType::Strapi => Color::Rgb(142, 118, 244), // strapi purple
            FrameworkType::NestJs => Color::Rgb(224, 35, 78),   // nest red
            FrameworkType::Express => Color::Rgb(200, 200, 200), // express gray
            FrameworkType::Hono => Color::Rgb(227, 96, 2),      // hono flame
            FrameworkType::Fastify => Color::Rgb(240, 240, 240), // fastify white
            FrameworkType::Elysia => Color::Rgb(236, 72, 153),  // elysia pink
            FrameworkType::Laravel => Color::Rgb(245, 82, 71),  // laravel red
            FrameworkType::Rails => Color::Rgb(204, 0, 0),      // rails crimson
            FrameworkType::Phoenix => Color::Rgb(253, 111, 34), // phoenix orange
            FrameworkType::FastAPI | FrameworkType::Flask | FrameworkType::Django => {
                Color::Rgb(55, 118, 171)
            }
            FrameworkType::CargoWatch => Color::Rgb(222, 95, 27),
            FrameworkType::Postgres => Color::Rgb(51, 103, 145),
            FrameworkType::Redis => Color::Rgb(210, 59, 38),
            FrameworkType::Mysql => Color::Rgb(0, 121, 168),
            FrameworkType::Sqlite => Color::Rgb(100, 180, 200),
            FrameworkType::None => Color::DarkGray,
        }
    }
}

/// High-level category: what *role* does this process play for a developer?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessCategory {
    /// Dev server with an active HTTP port (Next.js, Vite, etc.)
    DevServer,
    /// Build / watch tool (turbo, webpack, tsc, …)
    BuildTool,
    /// Database or cache process (postgres, redis, …)
    Database,
    /// A generic node/python/go process not matched to a framework
    RuntimeProcess,
    /// Everything else (kernel threads, system daemons, …)
    System,
}

impl ProcessCategory {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            ProcessCategory::DevServer => "DEV",
            ProcessCategory::BuildTool => "BUILD",
            ProcessCategory::Database => "DB",
            ProcessCategory::RuntimeProcess => "PROC",
            ProcessCategory::System => "",
        }
    }

    #[allow(dead_code)]
    pub fn color(self) -> Color {
        match self {
            ProcessCategory::DevServer => Color::Rgb(130, 210, 130),
            ProcessCategory::BuildTool => Color::Rgb(200, 190, 150),
            ProcessCategory::Database => Color::Rgb(130, 170, 210),
            ProcessCategory::RuntimeProcess => Color::Rgb(180, 180, 180),
            ProcessCategory::System => Color::DarkGray,
        }
    }
}

// ---------------------------------------------------------------------------
// Primary output type
// ---------------------------------------------------------------------------

/// All developer-context metadata attached to a [`crate::core::ProcessHarvest`].
#[derive(Clone, Debug)]
pub struct DeveloperMeta {
    pub runtime: RuntimeType,
    pub framework: FrameworkType,
    pub category: ProcessCategory,
    /// Friendly project name extracted from `package.json` `name` or `Cargo.toml` `[package].name`.
    pub project_name: Option<String>,
    /// Auto-generated URL for the dev server (populated when category == DevServer).
    pub dev_url: Option<String>,
}

impl DeveloperMeta {
    /// Convenience: no developer information found.
    #[allow(dead_code)]
    pub fn unknown() -> Self {
        Self {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::None,
            category: ProcessCategory::System,
            project_name: None,
            dev_url: None,
        }
    }

    /// Returns `true` when the process is a recognisable dev workload.
    pub fn is_dev_process(&self) -> bool {
        !matches!(self.category, ProcessCategory::System)
    }

    /// Short badge text — framework label if known, else runtime label.
    pub fn badge_label(&self) -> &'static str {
        if self.framework != FrameworkType::None {
            self.framework.label()
        } else {
            self.runtime.label()
        }
    }

    /// Badge colour (framework wins over runtime).
    pub fn badge_color(&self) -> Color {
        if self.framework != FrameworkType::None {
            self.framework.color()
        } else {
            self.runtime.color()
        }
    }
}

// ---------------------------------------------------------------------------
// Manifest cache
// ---------------------------------------------------------------------------

/// Cached result from reading a project directory.
struct ManifestEntry {
    project_name: Option<String>,
    framework_hint: Option<FrameworkType>,
    cached_at: Instant,
}

const MANIFEST_CACHE_TTL: Duration = Duration::from_secs(30);

/// Lightweight LRU-ish cache keyed by the CWD path string.
/// Uses `HashMap` with TTL eviction on access — good enough for a TUI refresh loop.
pub struct DetectorCache {
    manifests: HashMap<String, ManifestEntry>,
}

impl DetectorCache {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
        }
    }

    /// Look up (or probe) the manifest for `cwd`, returning `(project_name, framework_hint)`.
    pub fn probe_cwd(&mut self, cwd: &str) -> (Option<String>, Option<FrameworkType>) {
        // evict stale entry
        if let Some(e) = self.manifests.get(cwd) {
            if e.cached_at.elapsed() < MANIFEST_CACHE_TTL {
                return (e.project_name.clone(), e.framework_hint);
            }
        }

        let (project_name, framework_hint) = probe_manifest(cwd);
        self.manifests.insert(
            cwd.to_string(),
            ManifestEntry {
                project_name: project_name.clone(),
                framework_hint,
                cached_at: Instant::now(),
            },
        );
        (project_name, framework_hint)
    }

    /// Remove all entries older than TTL (call periodically to keep memory bounded).
    pub fn evict_stale(&mut self) {
        self.manifests
            .retain(|_, v| v.cached_at.elapsed() < MANIFEST_CACHE_TTL);
    }
}

// ---------------------------------------------------------------------------
// Manifest probing (Tier 2 — disk I/O, cached)
// ---------------------------------------------------------------------------

/// Read project files in `cwd` and return `(project_name, framework_hint)`.
fn probe_manifest(cwd: &str) -> (Option<String>, Option<FrameworkType>) {
    if cwd.is_empty() || cwd == "-" {
        return (None, None);
    }

    let path = Path::new(cwd);

    // --- package.json ---
    let pkg_path = path.join("package.json");
    let mut project_name = None;
    let mut framework = None;
    if pkg_path.exists() {
        if let Ok(contents) = fs::read_to_string(&pkg_path) {
            project_name = extract_json_string_field(&contents, "name");
            framework = detect_framework_from_package_json(&contents);
        }
    }

    // --- nuxt.config.* (common with pnpm when deps live only in the store) ---
    if framework.is_none()
        && (path.join("nuxt.config.ts").exists()
            || path.join("nuxt.config.js").exists()
            || path.join("nuxt.config.mjs").exists()
            || path.join("nuxt.config.cjs").exists())
    {
        framework = Some(FrameworkType::Nuxt);
    }

    if project_name.is_some() || framework.is_some() {
        return (project_name, framework);
    }

    // --- Cargo.toml ---
    let cargo_path = path.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(contents) = fs::read_to_string(&cargo_path) {
            let name = extract_toml_package_name(&contents);
            return (name, None);
        }
    }

    // --- pyproject.toml / setup.py → Python project ---
    let pyproject = path.join("pyproject.toml");
    let setup_py = path.join("setup.py");
    if pyproject.exists() || setup_py.exists() {
        let name = if pyproject.exists() {
            fs::read_to_string(&pyproject)
                .ok()
                .and_then(|c| extract_toml_package_name(&c))
        } else {
            None
        };
        return (name, None);
    }

    (None, None)
}

/// True when argv/exe looks like a Nuxt / Nuxi process (incl. pnpm's `.pnpm/nuxt@…` layout).
fn is_nuxt_cmd(cmd_lc: &str) -> bool {
    cmd_lc.contains("nuxt")
        || cmd_lc.contains("nuxi")
        || cmd_lc.contains("@nuxt/")
        || cmd_lc.contains(".nuxt/")
        || (cmd_lc.contains(".pnpm/") && cmd_lc.contains("nuxt@"))
        || cmd_lc.contains("/nuxt/bin/")
        || cmd_lc.contains("nuxt.mjs")
}

fn is_pkg_manager_invocation(exe_lc: &str, cmd_lc: &str) -> bool {
    exe_lc == "pnpm"
        || exe_lc.ends_with("/pnpm")
        || exe_lc == "npm"
        || exe_lc.ends_with("/npm")
        || exe_lc == "yarn"
        || exe_lc.ends_with("/yarn")
        || cmd_lc.starts_with("pnpm ")
        || cmd_lc.starts_with("npm ")
        || cmd_lc.starts_with("yarn ")
        || cmd_lc.contains("/pnpm.cjs")
        || cmd_lc.contains("/pnpm/bin/")
        || cmd_lc.contains("/npm/cli.js")
        || cmd_lc.contains("/yarn/bin/")
        || cmd_lc.contains("/yarn.cjs")
}

fn pkg_manager_category(cmd_lc: &str, fw: FrameworkType) -> ProcessCategory {
    if fw != FrameworkType::None {
        if is_http_dev_framework(fw) {
            ProcessCategory::DevServer
        } else {
            ProcessCategory::BuildTool
        }
    } else if cmd_lc.contains(" dev")
        || cmd_lc.contains(" run dev")
        || cmd_lc.contains(" serve")
        || cmd_lc.contains(" run serve")
    {
        // `pnpm dev` — framework resolved from manifest (package.json / nuxt.config).
        ProcessCategory::DevServer
    } else if cmd_lc.contains(" build") || cmd_lc.contains(" test") {
        ProcessCategory::BuildTool
    } else {
        ProcessCategory::RuntimeProcess
    }
}

/// Detect framework from `package.json` content by scanning `dependencies`/`devDependencies`.
fn detect_framework_from_package_json(json: &str) -> Option<FrameworkType> {
    // Check for config files first — more authoritative
    // Then fall back to dependency scanning.
    // We do a simple substring search to avoid pulling in a JSON parser.
    let deps_section = extract_json_block(json, "\"dependencies\"")
        .or_else(|| extract_json_block(json, "\"devDependencies\""));

    let all = match deps_section {
        Some(s) => s,
        None => json.to_string(),
    };

    if all.contains("\"next\"") {
        return Some(FrameworkType::NextJs);
    }
    if all.contains("\"@remix-run/") {
        return Some(FrameworkType::Remix);
    }
    if all.contains("\"nuxt\"") || all.contains("\"nuxt3\"") || all.contains("\"@nuxt/") {
        return Some(FrameworkType::Nuxt);
    }
    if all.contains("\"@sveltejs/kit\"") {
        return Some(FrameworkType::SvelteKit);
    }
    if all.contains("\"astro\"") {
        return Some(FrameworkType::Astro);
    }
    if all.contains("\"vite\"") {
        return Some(FrameworkType::Vite);
    }
    if all.contains("\"webpack\"") {
        return Some(FrameworkType::Webpack);
    }
    if all.contains("\"esbuild\"") {
        return Some(FrameworkType::Esbuild);
    }
    if all.contains("\"parcel\"") {
        return Some(FrameworkType::Parcel);
    }
    if all.contains("\"@strapi/") || all.contains("\"strapi\"") {
        return Some(FrameworkType::Strapi);
    }
    if all.contains("\"@nestjs/core\"") {
        return Some(FrameworkType::NestJs);
    }
    if all.contains("\"express\"") {
        return Some(FrameworkType::Express);
    }
    if all.contains("\"hono\"") || all.contains("\"@hono/") {
        return Some(FrameworkType::Hono);
    }
    if all.contains("\"fastify\"") {
        return Some(FrameworkType::Fastify);
    }
    if all.contains("\"elysia\"") {
        return Some(FrameworkType::Elysia);
    }
    if all.contains("\"fastapi\"") || all.contains("\"uvicorn\"") {
        return Some(FrameworkType::FastAPI);
    }
    if all.contains("\"flask\"") {
        return Some(FrameworkType::Flask);
    }
    if all.contains("\"django\"") {
        return Some(FrameworkType::Django);
    }
    None
}

/// Naively extract the `name` field value from a JSON string (no external crate).
fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    // Look for `"<field>": "value"` pattern
    let key = format!("\"{}\"", field);
    let idx = json.find(&key)?;
    let after = &json[idx + key.len()..];
    // skip whitespace and colon
    let colon_pos = after.find(':')?;
    let after_colon = after[colon_pos + 1..].trim_start();
    if after_colon.starts_with('"') {
        let inner = &after_colon[1..];
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        None
    }
}

/// Rudimentary: extract the first `{...}` block that follows `header` in `json`.
fn extract_json_block(json: &str, header: &str) -> Option<String> {
    let start = json.find(header)?;
    let brace = json[start..].find('{')? + start;
    let mut depth = 0usize;
    let bytes = json.as_bytes();
    for i in brace..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(json[brace..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract `[package] name = "..."` from a Cargo.toml string (no external TOML crate).
fn extract_toml_package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if in_package {
            if trimmed.starts_with('[') {
                break; // left [package] section
            }
            if trimmed.starts_with("name") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let val = trimmed[eq_pos + 1..].trim().trim_matches('"');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tier 1: CLI token matching
// ---------------------------------------------------------------------------

/// Quick and cheap classification based on the full command string and executable name.
struct CliMatch {
    runtime: RuntimeType,
    framework: FrameworkType,
    category: ProcessCategory,
}

fn match_cli(exe: &str, cmd: &str) -> Option<CliMatch> {
    let exe_lc = exe.to_lowercase();
    let cmd_lc = cmd.to_lowercase();

    // ── Databases / Infrastructure ──────────────────────────────────────
    if exe_lc.contains("postgres") || cmd_lc.contains("postgres") || exe_lc == "pg_ctl" {
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::Postgres,
            category: ProcessCategory::Database,
        });
    }
    if exe_lc.contains("redis-server") || cmd_lc.contains("redis-server") {
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::Redis,
            category: ProcessCategory::Database,
        });
    }
    if exe_lc.contains("mysqld") || cmd_lc.contains("mysqld") {
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::Mysql,
            category: ProcessCategory::Database,
        });
    }
    if exe_lc.contains("sqlite") || cmd_lc.contains("sqlite") {
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::Sqlite,
            category: ProcessCategory::Database,
        });
    }

    // ── Bun ─────────────────────────────────────────────────────────────
    if exe_lc == "bun"
        || exe_lc.ends_with("/bun")
        || exe_lc.contains("/.bun/")
        || cmd_lc.starts_with("bun ")
        || cmd_lc.starts_with("bunx ")
        || cmd_lc == "bun"
    {
        // Try to detect framework from cmd
        let fw = if cmd_lc.contains("next") {
            FrameworkType::NextJs
        } else if cmd_lc.contains("vite") {
            FrameworkType::Vite
        } else if cmd_lc.contains("astro") {
            FrameworkType::Astro
        } else if cmd_lc.contains("remix") || cmd_lc.contains("@remix-run") {
            FrameworkType::Remix
        } else if is_nuxt_cmd(&cmd_lc) {
            FrameworkType::Nuxt
        } else if cmd_lc.contains("svelte") || cmd_lc.contains("@sveltejs") {
            FrameworkType::SvelteKit
        } else {
            FrameworkType::None
        };
        let cat = if fw != FrameworkType::None
            || cmd_lc.contains(" dev")
            || cmd_lc.contains("--watch")
            || cmd_lc.contains("--hot")
        {
            ProcessCategory::DevServer
        } else if cmd_lc.contains(" build") || cmd_lc.contains(" test") {
            ProcessCategory::BuildTool
        } else {
            ProcessCategory::RuntimeProcess
        };
        return Some(CliMatch {
            runtime: RuntimeType::Bun,
            framework: fw,
            category: cat,
        });
    }

    // ── Deno ────────────────────────────────────────────────────────────
    if exe_lc == "deno" || exe_lc.ends_with("/deno") {
        return Some(CliMatch {
            runtime: RuntimeType::Deno,
            framework: FrameworkType::None,
            category: ProcessCategory::RuntimeProcess,
        });
    }

    // ── Cargo / Rust ─────────────────────────────────────────────────────
    if exe_lc == "cargo" || exe_lc.ends_with("/cargo") {
        return Some(CliMatch {
            runtime: RuntimeType::Rust,
            framework: FrameworkType::CargoWatch,
            category: ProcessCategory::BuildTool,
        });
    }
    if cmd_lc.contains("cargo watch") || cmd_lc.contains("cargo-watch") {
        return Some(CliMatch {
            runtime: RuntimeType::Rust,
            framework: FrameworkType::CargoWatch,
            category: ProcessCategory::BuildTool,
        });
    }

    // ── Turborepo ────────────────────────────────────────────────────────
    if exe_lc == "turbo"
        || exe_lc.ends_with("/turbo")
        || cmd_lc.contains("turbo run")
        || cmd_lc.contains("turbo dev")
    {
        return Some(CliMatch {
            runtime: RuntimeType::Node,
            framework: FrameworkType::Turborepo,
            category: ProcessCategory::BuildTool,
        });
    }

    // ── Wrangler (Cloudflare) ────────────────────────────────────────────
    if cmd_lc.contains("wrangler") {
        return Some(CliMatch {
            runtime: RuntimeType::Node,
            framework: FrameworkType::Wrangler,
            category: ProcessCategory::DevServer,
        });
    }

    // ── Node.js with framework detection ────────────────────────────────
    if exe_lc == "node" || exe_lc.ends_with("/node") || exe_lc == "nodejs" {
        // pnpm/npm/yarn are shell scripts that exec `node …/pnpm.cjs run dev`
        if is_pkg_manager_invocation(&exe_lc, &cmd_lc) {
            let fw = detect_node_framework(&cmd_lc);
            return Some(CliMatch {
                runtime: RuntimeType::Node,
                framework: fw,
                category: pkg_manager_category(&cmd_lc, fw),
            });
        }

        let fw = detect_node_framework(&cmd_lc);
        let cat = match fw {
            FrameworkType::NextJs
            | FrameworkType::Vite
            | FrameworkType::Astro
            | FrameworkType::Nuxt
            | FrameworkType::Remix
            | FrameworkType::SvelteKit
            | FrameworkType::Wrangler
            | FrameworkType::Strapi
            | FrameworkType::NestJs
            | FrameworkType::Express
            | FrameworkType::Hono
            | FrameworkType::Fastify
            | FrameworkType::Elysia => ProcessCategory::DevServer,
            FrameworkType::Turborepo | FrameworkType::Webpack | FrameworkType::Esbuild => {
                ProcessCategory::BuildTool
            }
            _ => ProcessCategory::RuntimeProcess,
        };
        return Some(CliMatch {
            runtime: RuntimeType::Node,
            framework: fw,
            category: cat,
        });
    }

    // ── Python ──────────────────────────────────────────────────────────
    if exe_lc.starts_with("python") || exe_lc.ends_with("/python3") || exe_lc.ends_with("/python") {
        let fw = if cmd_lc.contains("uvicorn") || cmd_lc.contains("fastapi") {
            FrameworkType::FastAPI
        } else if cmd_lc.contains("flask") {
            FrameworkType::Flask
        } else if cmd_lc.contains("django") || cmd_lc.contains("manage.py") {
            FrameworkType::Django
        } else {
            FrameworkType::None
        };
        let cat = if fw != FrameworkType::None {
            ProcessCategory::DevServer
        } else {
            ProcessCategory::RuntimeProcess
        };
        return Some(CliMatch {
            runtime: RuntimeType::Python,
            framework: fw,
            category: cat,
        });
    }
    if exe_lc == "uvicorn" || exe_lc.ends_with("/uvicorn") {
        return Some(CliMatch {
            runtime: RuntimeType::Python,
            framework: FrameworkType::FastAPI,
            category: ProcessCategory::DevServer,
        });
    }
    if exe_lc == "gunicorn" || exe_lc.ends_with("/gunicorn") {
        return Some(CliMatch {
            runtime: RuntimeType::Python,
            framework: FrameworkType::Flask,
            category: ProcessCategory::DevServer,
        });
    }

    // ── PHP / Laravel ───────────────────────────────────────────────────
    if exe_lc.starts_with("php") || cmd_lc.contains("artisan serve") {
        let fw = if cmd_lc.contains("artisan") {
            FrameworkType::Laravel
        } else {
            FrameworkType::None
        };
        let cat = if cmd_lc.contains("serve") || cmd_lc.contains("-s") {
            ProcessCategory::DevServer
        } else {
            ProcessCategory::RuntimeProcess
        };
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: fw,
            category: cat,
        });
    }

    // ── Ruby on Rails ───────────────────────────────────────────────────
    if exe_lc.starts_with("ruby")
        || exe_lc == "rails"
        || exe_lc == "puma"
        || cmd_lc.contains("rails s")
        || cmd_lc.contains("puma")
    {
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: FrameworkType::Rails,
            category: ProcessCategory::DevServer,
        });
    }

    // ── Elixir / Phoenix ────────────────────────────────────────────────
    if exe_lc == "beam.smp" || exe_lc == "elixir" || cmd_lc.contains("phx.server") {
        let fw = if cmd_lc.contains("phx") {
            FrameworkType::Phoenix
        } else {
            FrameworkType::None
        };
        let cat = if cmd_lc.contains("phx.server") {
            ProcessCategory::DevServer
        } else {
            ProcessCategory::RuntimeProcess
        };
        return Some(CliMatch {
            runtime: RuntimeType::Unknown,
            framework: fw,
            category: cat,
        });
    }

    // ── Go ───────────────────────────────────────────────────────────────
    if exe_lc == "go" || exe_lc.ends_with("/go") || exe_lc == "air" || cmd_lc.contains("air") {
        let is_dev = cmd_lc.contains("run") || exe_lc == "air" || cmd_lc.contains("air");
        return Some(CliMatch {
            runtime: RuntimeType::Go,
            framework: FrameworkType::None,
            category: if is_dev {
                ProcessCategory::DevServer
            } else {
                ProcessCategory::RuntimeProcess
            },
        });
    }

    None
}

/// Detect Node.js framework from command arguments.
fn detect_node_framework(cmd_lc: &str) -> FrameworkType {
    // Nuxt bundles Vite — check Nuxt markers before generic `vite`.
    if is_nuxt_cmd(cmd_lc) {
        return FrameworkType::Nuxt;
    }
    if cmd_lc.contains("next")
        && (cmd_lc.contains("dev") || cmd_lc.contains("start") || cmd_lc.contains("next-server"))
    {
        FrameworkType::NextJs
    } else if cmd_lc.contains("next") {
        FrameworkType::NextJs
    } else if cmd_lc.contains("vite") {
        FrameworkType::Vite
    } else if cmd_lc.contains("astro") {
        FrameworkType::Astro
    } else if cmd_lc.contains("remix") || cmd_lc.contains("@remix-run") {
        FrameworkType::Remix
    } else if cmd_lc.contains("svelte") || cmd_lc.contains("@sveltejs") {
        FrameworkType::SvelteKit
    } else if cmd_lc.contains("wrangler") || cmd_lc.contains("workerd") {
        FrameworkType::Wrangler
    } else if cmd_lc.contains("strapi") {
        FrameworkType::Strapi
    } else if cmd_lc.contains("nest") {
        FrameworkType::NestJs
    } else if cmd_lc.contains("express") {
        FrameworkType::Express
    } else if cmd_lc.contains("hono") {
        FrameworkType::Hono
    } else if cmd_lc.contains("fastify") {
        FrameworkType::Fastify
    } else if cmd_lc.contains("elysia") {
        FrameworkType::Elysia
    } else if cmd_lc.contains("turbo") {
        FrameworkType::Turborepo
    } else if cmd_lc.contains("webpack") {
        FrameworkType::Webpack
    } else if cmd_lc.contains("esbuild") {
        FrameworkType::Esbuild
    } else if cmd_lc.contains("parcel") {
        FrameworkType::Parcel
    } else {
        FrameworkType::None
    }
}

// ---------------------------------------------------------------------------
// Tier 3: Port correlation
// ---------------------------------------------------------------------------

/// Ports reserved for databases or infrastructure daemons that are not HTTP dev servers.
const DB_INFRA_PORTS: &[u16] = &[
    5432, // PostgreSQL
    6379, 6380, // Redis
    3306, 33060, // MySQL / MariaDB
    27017, 27018, // MongoDB
    9200, 9300,  // Elasticsearch
    11211, // Memcached
    5672, 15672, // RabbitMQ
    2181, 9092, // Zookeeper / Kafka
];

/// User-facing HTTP/dev port: not a database, not a kernel ephemeral `listen(0)` socket.
pub fn is_dev_port(port: u16) -> bool {
    if DB_INFRA_PORTS.contains(&port) {
        return false;
    }
    // Kernel ephemeral sockets from listen(0), typically 32768-60999 on Linux.
    if port >= 32768 {
        return false;
    }
    port >= 1024 || port == 80 || port == 443 || port == 808
}

/// Frameworks that actually serve HTTP. Workers inherit the same argv/cwd as
/// `next dev` but must not be labelled DevServer unless they listen.
fn is_http_dev_framework(fw: FrameworkType) -> bool {
    matches!(
        fw,
        FrameworkType::NextJs
            | FrameworkType::Vite
            | FrameworkType::Astro
            | FrameworkType::Nuxt
            | FrameworkType::Remix
            | FrameworkType::SvelteKit
            | FrameworkType::Wrangler
            | FrameworkType::Strapi
            | FrameworkType::NestJs
            | FrameworkType::Express
            | FrameworkType::Hono
            | FrameworkType::Fastify
            | FrameworkType::Elysia
            | FrameworkType::Laravel
            | FrameworkType::Rails
            | FrameworkType::Phoenix
            | FrameworkType::FastAPI
            | FrameworkType::Flask
            | FrameworkType::Django
    )
}

/// True for the CLI that *starts* the server (`next dev`, `vite`, …), not forked workers.
fn is_http_server_entrypoint(cmd: &str, fw: FrameworkType) -> bool {
    let c = cmd.to_lowercase();
    match fw {
        FrameworkType::NextJs => {
            c.contains("next dev")
                || c.contains("next start")
                || c.contains(".bin/next")
                || c.contains("/bin/next")
        }
        FrameworkType::Vite => c.contains("vite"),
        FrameworkType::Astro => c.contains("astro"),
        FrameworkType::Nuxt => {
            is_nuxt_cmd(&c)
                || (c.contains("pnpm") && (c.contains(" dev") || c.contains(" run dev")))
                || c.contains("npm run dev")
                || c.contains("yarn dev")
        }
        FrameworkType::Remix => c.contains("remix"),
        FrameworkType::SvelteKit => c.contains("svelte-kit") || c.contains("vite"),
        FrameworkType::Wrangler => c.contains("wrangler"),
        FrameworkType::Strapi => c.contains("strapi"),
        FrameworkType::NestJs => c.contains("nest"),
        FrameworkType::Express => c.contains("express"),
        FrameworkType::Hono => c.contains("hono"),
        FrameworkType::Fastify => c.contains("fastify"),
        FrameworkType::Elysia => c.contains("elysia"),
        FrameworkType::Laravel => c.contains("artisan"),
        FrameworkType::Rails => c.contains("rails") || c.contains("puma"),
        FrameworkType::Phoenix => c.contains("phx.server"),
        FrameworkType::FastAPI => c.contains("uvicorn") || c.contains("fastapi"),
        FrameworkType::Flask => c.contains("flask") || c.contains("gunicorn"),
        FrameworkType::Django => c.contains("manage.py") || c.contains("django"),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Main classification entry point
// ---------------------------------------------------------------------------

/// Classify a single process given its executable, command string, working dir,
/// and list of listening ports.  Uses the cache for manifest lookups.
///
/// # Arguments
/// * `exe`   — process executable path
/// * `cmd`   — full command line string (argv joined with spaces)
/// * `cwd`   — working directory (may be empty / "-")
/// * `ports` — TCP/UDP ports the process is currently listening on
/// * `cache` — shared manifest cache (call `.evict_stale()` periodically)
pub fn classify_process(
    exe: &str,
    cmd: &str,
    cwd: &str,
    ports: &[u16],
    cache: &mut DetectorCache,
) -> DeveloperMeta {
    // Tier 1: CLI matching
    let cli = match_cli(exe, cmd);

    let (mut runtime, mut framework, mut category) = match &cli {
        Some(m) => (m.runtime, m.framework, m.category),
        None => (
            RuntimeType::Unknown,
            FrameworkType::None,
            ProcessCategory::System,
        ),
    };

    // Tier 2: manifest probing (only if we have a useful cwd)
    let (project_name, manifest_fw) = if !cwd.is_empty() && cwd != "-" {
        cache.probe_cwd(cwd)
    } else {
        (None, None)
    };

    // Manifest can sharpen the framework if CLI didn't resolve it,
    // but only for actual runtimes or processes with listening ports.
    if framework == FrameworkType::None {
        if let Some(mfw) = manifest_fw {
            let is_dev_runtime = runtime != RuntimeType::Unknown || !ports.is_empty();
            if is_dev_runtime {
                framework = mfw;
                if category == ProcessCategory::RuntimeProcess
                    || category == ProcessCategory::System
                {
                    category = if ports.iter().any(|p| is_dev_port(*p)) {
                        ProcessCategory::DevServer
                    } else if is_http_dev_framework(mfw) {
                        // Same-cwd Next/Vite workers share package.json — not extra servers.
                        ProcessCategory::RuntimeProcess
                    } else {
                        ProcessCategory::BuildTool
                    };
                }
            }
        }
    }

    // Nuxt dev spawns Vite child processes — package.json says Nuxt, argv says Vite.
    if framework == FrameworkType::Vite {
        if manifest_fw == Some(FrameworkType::Nuxt) {
            framework = FrameworkType::Nuxt;
        }
    }

    // Tier 3: port correlation — upgrade category to DevServer if applicable
    if !ports.is_empty() && category != ProcessCategory::Database {
        let has_dev_port = ports.iter().any(|p| is_dev_port(*p));
        if has_dev_port {
            category = ProcessCategory::DevServer;
        }
    }

    // Forked workers inherit argv/cwd (e.g. `next-server`) but do not listen
    // on a user-facing port — they often bind an ephemeral `listen(0)` socket.
    // Keep the CLI entrypoint (`next dev`) visible while the HTTP port is still binding.
    let has_public_port = ports.iter().any(|p| is_dev_port(*p));
    if category == ProcessCategory::DevServer
        && !has_public_port
        && !is_http_server_entrypoint(cmd, framework)
    {
        category = ProcessCategory::RuntimeProcess;
    }

    // Build dev URL from the lowest dev port if this is a dev server
    let dev_url = if category == ProcessCategory::DevServer {
        ports
            .iter()
            .find(|&&p| is_dev_port(p))
            .map(|p| format!("http://localhost:{}", p))
    } else {
        None
    };

    // If still unknown runtime but we matched something, default to Node for JS frameworks
    if runtime == RuntimeType::Unknown {
        match framework {
            FrameworkType::NextJs
            | FrameworkType::Vite
            | FrameworkType::Astro
            | FrameworkType::Nuxt
            | FrameworkType::Remix
            | FrameworkType::SvelteKit
            | FrameworkType::Turborepo
            | FrameworkType::Webpack
            | FrameworkType::Esbuild
            | FrameworkType::Parcel => {
                runtime = RuntimeType::Node;
            }
            FrameworkType::FastAPI | FrameworkType::Flask | FrameworkType::Django => {
                runtime = RuntimeType::Python;
            }
            FrameworkType::CargoWatch => {
                runtime = RuntimeType::Rust;
            }
            _ => {}
        }
    }

    DeveloperMeta {
        runtime,
        framework,
        category,
        project_name,
        dev_url,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> DetectorCache {
        DetectorCache::new()
    }

    // ── CLI tier ────────────────────────────────────────────────────────

    #[test]
    fn test_detect_nextjs_via_cli() {
        let meta = classify_process(
            "node",
            "/app/node_modules/.bin/next dev",
            "",
            &[3000],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::NextJs);
        assert_eq!(meta.runtime, RuntimeType::Node);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, Some("http://localhost:3000".to_string()));
    }

    #[test]
    fn test_nextjs_worker_without_listen_port_is_not_a_server() {
        let meta = classify_process("node", "next-server (v15.1.0)", "", &[], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::NextJs);
        assert_eq!(meta.category, ProcessCategory::RuntimeProcess);
        assert_eq!(meta.dev_url, None);
    }

    #[test]
    fn test_nextjs_cli_without_port_stays_dev_server() {
        let meta = classify_process(
            "node",
            "/app/node_modules/.bin/next dev",
            "",
            &[],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::NextJs);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, None);
    }

    #[test]
    fn test_detect_vite_via_cli() {
        let meta = classify_process("node", "vite --port 5173", "", &[5173], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::Vite);
        assert_eq!(meta.runtime, RuntimeType::Node);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, Some("http://localhost:5173".to_string()));
    }

    #[test]
    fn test_detect_nuxt_via_pnpm_store_path() {
        let cmd =
            "node /home/dev/app/node_modules/.pnpm/nuxt@3.15.0/node_modules/nuxt/bin/nuxt.mjs dev";
        let meta = classify_process("node", cmd, "", &[3000], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::Nuxt);
        assert_eq!(meta.category, ProcessCategory::DevServer);
    }

    #[test]
    fn test_detect_nuxt_via_nuxi() {
        let cmd = "node /home/dev/.pnpm/@nuxt+cli@3.22.0/node_modules/@nuxt/cli/bin/nuxi.mjs dev";
        let meta = classify_process("node", cmd, "", &[3000], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::Nuxt);
    }

    #[test]
    fn test_nuxt_vite_worker_prefers_nuxt_from_manifest() {
        let cwd = std::env::temp_dir().join(format!("devtop-nuxt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&cwd);
        std::fs::write(
            cwd.join("package.json"),
            r#"{"name":"my-nuxt","dependencies":{"nuxt":"^3.15.0"}}"#,
        )
        .unwrap();

        let meta = classify_process(
            "node",
            "node node_modules/vite/bin/vite.js --port 3000",
            cwd.to_str().unwrap(),
            &[3000],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::Nuxt);
        assert_eq!(meta.project_name, Some("my-nuxt".to_string()));

        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn test_pnpm_dev_without_nuxt_in_argv_uses_manifest() {
        let cwd = std::env::temp_dir().join(format!("devtop-pnpm-nuxt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&cwd);
        std::fs::write(
            cwd.join("package.json"),
            r#"{"name":"www","scripts":{"dev":"nuxt dev"},"devDependencies":{"nuxt":"^3.15.0"}}"#,
        )
        .unwrap();

        let cmd = "node /home/dev/.local/share/pnpm/.tools/pnpm/9.15.0/bin/pnpm.cjs run dev";
        let meta = classify_process("node", cmd, cwd.to_str().unwrap(), &[], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::Nuxt);
        assert_eq!(meta.category, ProcessCategory::DevServer);

        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn test_nuxt_cmd_beats_vite_in_argv() {
        assert!(is_nuxt_cmd("node .nuxt/dist/server/index.mjs && vite"));
        let meta = classify_process(
            "node",
            "node /app/.nuxt/dev/index.mjs",
            "",
            &[3000],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::Nuxt);
    }

    #[test]
    fn test_detect_wrangler() {
        let meta = classify_process(
            "node",
            "wrangler dev --port 8787",
            "",
            &[8787],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::Wrangler);
        assert_eq!(meta.category, ProcessCategory::DevServer);
    }

    #[test]
    fn test_detect_turborepo() {
        let meta = classify_process("turbo", "turbo dev", "", &[], &mut make_cache());
        assert_eq!(meta.framework, FrameworkType::Turborepo);
        assert_eq!(meta.category, ProcessCategory::BuildTool);
    }

    #[test]
    fn test_detect_bun_with_next() {
        let meta = classify_process("bun", "bun run next dev", "", &[3000], &mut make_cache());
        assert_eq!(meta.runtime, RuntimeType::Bun);
        assert_eq!(meta.framework, FrameworkType::NextJs);
        assert_eq!(meta.category, ProcessCategory::DevServer);
    }

    #[test]
    fn test_detect_bun_generic() {
        let meta = classify_process("bun", "bun run index.ts", "", &[], &mut make_cache());
        assert_eq!(meta.runtime, RuntimeType::Bun);
        assert_eq!(meta.framework, FrameworkType::None);
        assert_eq!(meta.category, ProcessCategory::RuntimeProcess);
    }

    #[test]
    fn test_detect_bun_serve_with_port() {
        let meta = classify_process(
            "/home/user/.bun/bin/bun",
            "bun --watch src/server.ts",
            "",
            &[3000],
            &mut make_cache(),
        );
        assert_eq!(meta.runtime, RuntimeType::Bun);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, Some("http://localhost:3000".to_string()));
        assert_eq!(meta.badge_label(), "Bun");
    }

    #[test]
    fn test_detect_bun_on_custom_port_1337() {
        let meta = classify_process(
            "bun",
            "bun run src/index.ts",
            "",
            &[1337],
            &mut make_cache(),
        );
        assert_eq!(meta.runtime, RuntimeType::Bun);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, Some("http://localhost:1337".to_string()));
    }

    #[test]
    fn test_detect_strapi_port_1337() {
        let meta = classify_process("node", "strapi develop", "", &[1337], &mut make_cache());
        assert_eq!(meta.runtime, RuntimeType::Node);
        assert_eq!(meta.framework, FrameworkType::Strapi);
        assert_eq!(meta.category, ProcessCategory::DevServer);
        assert_eq!(meta.dev_url, Some("http://localhost:1337".to_string()));
        assert_eq!(meta.badge_label(), "Strapi");
    }

    #[test]
    fn test_detect_deno() {
        let meta = classify_process(
            "deno",
            "deno run --allow-net server.ts",
            "",
            &[8000],
            &mut make_cache(),
        );
        assert_eq!(meta.runtime, RuntimeType::Deno);
    }

    #[test]
    fn test_detect_cargo() {
        let meta = classify_process("cargo", "cargo watch -x run", "", &[], &mut make_cache());
        assert_eq!(meta.runtime, RuntimeType::Rust);
        assert_eq!(meta.framework, FrameworkType::CargoWatch);
        assert_eq!(meta.category, ProcessCategory::BuildTool);
    }

    #[test]
    fn test_detect_postgres() {
        let meta = classify_process(
            "postgres",
            "postgres -D /var/lib/postgresql/data",
            "",
            &[5432],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::Postgres);
        assert_eq!(meta.category, ProcessCategory::Database);
    }

    #[test]
    fn test_detect_redis() {
        let meta = classify_process(
            "redis-server",
            "redis-server /etc/redis.conf",
            "",
            &[6379],
            &mut make_cache(),
        );
        assert_eq!(meta.framework, FrameworkType::Redis);
        assert_eq!(meta.category, ProcessCategory::Database);
    }

    #[test]
    fn test_detect_python_uvicorn() {
        let meta = classify_process(
            "uvicorn",
            "uvicorn main:app --reload",
            "",
            &[8000],
            &mut make_cache(),
        );
        assert_eq!(meta.runtime, RuntimeType::Python);
        assert_eq!(meta.framework, FrameworkType::FastAPI);
        assert_eq!(meta.category, ProcessCategory::DevServer);
    }

    #[test]
    fn test_unknown_process_is_system() {
        let meta = classify_process("kworker", "kworker/0:1", "", &[], &mut make_cache());
        assert_eq!(meta.category, ProcessCategory::System);
        assert_eq!(meta.framework, FrameworkType::None);
        assert!(!meta.is_dev_process());
    }

    #[test]
    fn test_node_without_framework_is_runtime_process() {
        let meta = classify_process("node", "node server.js", "", &[], &mut make_cache());
        assert_eq!(meta.runtime, RuntimeType::Node);
        assert_eq!(meta.framework, FrameworkType::None);
        assert_eq!(meta.category, ProcessCategory::RuntimeProcess);
    }

    // ── Port tier ───────────────────────────────────────────────────────

    #[test]
    fn test_dev_port_upgrades_category() {
        // A bare `node` process on port 3000 should be promoted to DevServer
        let meta = classify_process("node", "node .", "", &[3000], &mut make_cache());
        assert_eq!(meta.category, ProcessCategory::DevServer);
    }

    #[test]
    fn test_ephemeral_port_is_not_a_dev_server() {
        assert!(!is_dev_port(33169));
        assert!(!is_dev_port(33787));
        assert!(is_dev_port(3000));
        assert!(is_dev_port(1337));

        let meta = classify_process("node", "node .", "", &[33169], &mut make_cache());
        assert_eq!(meta.category, ProcessCategory::RuntimeProcess);
        assert_eq!(meta.dev_url, None);
    }

    #[test]
    fn test_database_port_does_not_upgrade() {
        // postgres on 5432 should stay Database, not get promoted again
        let meta = classify_process("postgres", "postgres", "", &[5432], &mut make_cache());
        assert_eq!(meta.category, ProcessCategory::Database);
    }

    // ── Manifest tier ───────────────────────────────────────────────────

    #[test]
    fn test_json_field_extraction() {
        let json = r#"{"name":"my-app","version":"1.0.0"}"#;
        assert_eq!(
            extract_json_string_field(json, "name"),
            Some("my-app".to_string())
        );
        assert_eq!(
            extract_json_string_field(json, "version"),
            Some("1.0.0".to_string())
        );
        assert_eq!(extract_json_string_field(json, "missing"), None);
    }

    #[test]
    fn test_detect_nextjs_from_package_json_deps() {
        let json = r#"{"name":"sora-ui","dependencies":{"next":"14.0.0","react":"18.0.0"}}"#;
        assert_eq!(
            detect_framework_from_package_json(json),
            Some(FrameworkType::NextJs)
        );
    }

    #[test]
    fn test_detect_vite_from_package_json_devdeps() {
        let json = r#"{"name":"playground","devDependencies":{"vite":"5.0.0"}}"#;
        assert_eq!(
            detect_framework_from_package_json(json),
            Some(FrameworkType::Vite)
        );
    }

    #[test]
    fn test_detect_astro_from_package_json() {
        let json = r#"{"name":"docs","dependencies":{"astro":"3.0.0"}}"#;
        assert_eq!(
            detect_framework_from_package_json(json),
            Some(FrameworkType::Astro)
        );
    }

    #[test]
    fn test_toml_package_name_extraction() {
        let toml = "[package]\nname = \"devtop\"\nversion = \"0.1.0\"\n";
        assert_eq!(extract_toml_package_name(toml), Some("devtop".to_string()));
    }

    #[test]
    fn test_toml_package_name_missing() {
        let toml = "[dependencies]\nratatui = \"0.30\"\n";
        assert_eq!(extract_toml_package_name(toml), None);
    }

    // ── Badge helpers ───────────────────────────────────────────────────

    #[test]
    fn test_badge_label_prefers_framework() {
        let meta = DeveloperMeta {
            runtime: RuntimeType::Node,
            framework: FrameworkType::NextJs,
            category: ProcessCategory::DevServer,
            project_name: None,
            dev_url: None,
        };
        assert_eq!(meta.badge_label(), "Next.js");
    }

    #[test]
    fn test_badge_label_falls_back_to_runtime() {
        let meta = DeveloperMeta {
            runtime: RuntimeType::Bun,
            framework: FrameworkType::None,
            category: ProcessCategory::RuntimeProcess,
            project_name: None,
            dev_url: None,
        };
        assert_eq!(meta.badge_label(), "Bun");
    }

    #[test]
    fn test_is_dev_process() {
        let dev = DeveloperMeta {
            runtime: RuntimeType::Node,
            framework: FrameworkType::Vite,
            category: ProcessCategory::DevServer,
            project_name: None,
            dev_url: None,
        };
        assert!(dev.is_dev_process());

        let sys = DeveloperMeta::unknown();
        assert!(!sys.is_dev_process());
    }

    #[test]
    fn test_process_category_labels_and_colors() {
        assert_eq!(ProcessCategory::DevServer.label(), "DEV");
        assert_eq!(ProcessCategory::BuildTool.label(), "BUILD");
        assert_eq!(ProcessCategory::Database.label(), "DB");
        assert_eq!(ProcessCategory::RuntimeProcess.label(), "PROC");
        assert_eq!(ProcessCategory::System.label(), "");

        let _ = ProcessCategory::DevServer.color();
        let _ = FrameworkType::Vite.color();
        let _ = RuntimeType::Rust.color();
    }
}
