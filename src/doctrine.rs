//! Doctrine markdown loader — resolves and caches rank/MOS/protocol markdown.
//!
//! Path resolution (in order):
//!   1. `RR_DOCTRINE_PATH` — explicit env override (used by tests and installers)
//!   2. `~/.rational-reserve/doctrine/` — runtime-installed copy (production path)
//!   3. Packaged fallback — shipped inside the binary as compiled-in strings

use crate::models::{CivilianTier, MOSEnum, RankEnum};
use crate::traits::CivilianDoctrine;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DoctrineError {
    #[error("doctrine file not found: {path}")]
    NotFound { path: String },

    #[error(
        "No doctrine directory found. Checked $RR_DOCTRINE_PATH, ~/.rational-reserve/doctrine/"
    )]
    NoDoctrineRoot,

    #[error("malformed frontmatter in {path}: {reason}")]
    MalformedFrontmatter { path: String, reason: String },

    #[error("invalid slug `{slug}`: must match [a-z0-9][a-z0-9_-]* and be 1-64 chars")]
    InvalidSlug { slug: String },
}

/// Validate that a slug is safe to embed in a filesystem path.
///
/// Accepts only ASCII `[a-z0-9_-]`, must start with `[a-z0-9]`, length 1-64.
/// Rejects anything containing `/`, `\`, `.`, `..`, NUL, or any non-matching byte
/// — blocks path traversal (CWE-22) before the slug reaches `PathBuf::join`.
fn validate_slug(slug: &str) -> std::result::Result<(), DoctrineError> {
    if slug.is_empty() || slug.len() > 64 {
        return Err(DoctrineError::InvalidSlug { slug: slug.to_string() });
    }
    let mut chars = slug.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err(DoctrineError::InvalidSlug { slug: slug.to_string() });
    }
    for c in chars {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
        if !ok {
            return Err(DoctrineError::InvalidSlug { slug: slug.to_string() });
        }
    }
    Ok(())
}

pub type Result<T> = std::result::Result<T, DoctrineError>;

/// Default `DoctrineLoader` implementation — loads markdown from filesystem.
#[derive(Clone)]
pub struct FileSystemDoctrineLoader {
    root: PathBuf,
    cache: Arc<Mutex<HashMap<PathBuf, String>>>,
    version: Arc<Mutex<Option<String>>>,
}

impl FileSystemDoctrineLoader {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: Arc::new(Mutex::new(HashMap::new())),
            version: Arc::new(Mutex::new(None)),
        }
    }

    fn read_required(&self, path: &PathBuf) -> Result<String> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(path) {
                return Ok(cached.clone());
            }
        }

        if !path.is_file() {
            return Err(DoctrineError::NotFound {
                path: path.display().to_string(),
            });
        }

        // Defense in depth (CWE-22): even though every caller builds `path`
        // from the doctrine root + validated slugs, verify that the canonical
        // resolved path is still rooted at `self.root` before reading. A
        // symlink or unexpected `..` component that survived upstream
        // validation is refused here rather than silently followed.
        self.ensure_within_root(path)?;

        // Open the already-validated file via an explicit File handle and read
        // its contents. Path has been (a) constructed only from doctrine-root +
        // slugs validated by `validate_slug`, and (b) canonicalized + checked
        // against the doctrine root above. No caller-controlled traversal is
        // reachable from here.
        let mut file = std::fs::File::open(path).map_err(|e| DoctrineError::NotFound { // nosemgrep
            path: format!("{}: {}", path.display(), e),
        })?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| DoctrineError::NotFound {
            path: format!("{}: {}", path.display(), e),
        })?;

        let mut cache = self.cache.lock().unwrap();
        cache.insert(path.clone(), content.clone());
        Ok(content)
    }

    /// Verify that `path` (after canonicalization) is still contained within
    /// `self.root`. Returns [`DoctrineError::NotFound`] if the path escapes
    /// the configured doctrine root — this is a hard path-traversal guard.
    fn ensure_within_root(&self, path: &PathBuf) -> Result<()> {
        let canon_path = path.canonicalize().map_err(|e| DoctrineError::NotFound {
            path: format!("{}: canonicalize failed: {}", path.display(), e),
        })?;
        let canon_root = self.root.canonicalize().map_err(|e| DoctrineError::NotFound {
            path: format!("{}: root canonicalize failed: {}", self.root.display(), e),
        })?;
        if !canon_path.starts_with(&canon_root) {
            return Err(DoctrineError::NotFound {
                path: format!(
                    "path `{}` escapes doctrine root `{}`",
                    canon_path.display(),
                    canon_root.display()
                ),
            });
        }
        Ok(())
    }
}

impl crate::traits::DoctrineLoader for FileSystemDoctrineLoader {
    fn version(&self) -> String {
        let mut ver = self.version.lock().unwrap();
        if ver.is_none() {
            let path = self.root.join("VERSION");
            *ver = std::fs::read_to_string(&path).ok().map(|s| s.trim().to_string());
        }
        ver.clone().unwrap_or_else(|| "0.0.0".to_string())
    }

    fn rank_doctrine(&self, rank: &RankEnum) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let filename = format!("{}.md", rank.full_name().to_lowercase().replace(' ', "-"));
        Ok(self.read_required(&self.root.join("ranks").join(&filename))?)
    }

    fn mos_doctrine(&self, mos: &MOSEnum) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let slug = mos.full_name().to_lowercase().replace(' ', "-");
        let mos_code = match mos {
            MOSEnum::IntelAnalyst => "35f",
            MOSEnum::CounterIntel => "35l",
            MOSEnum::SigintAnalyst => "35n",
            MOSEnum::Infantry => "11b",
            MOSEnum::CombatEngineer => "12b",
            MOSEnum::Artillery => "13b",
            MOSEnum::CavalryScout => "19d",
            MOSEnum::ItSpecialist => "25b",
            MOSEnum::HrSpecialist => "42a",
            MOSEnum::Transport => "88m",
            MOSEnum::Logistics => "92g",
            MOSEnum::SfEngineer => "18b",
            MOSEnum::Ranger => "75r",
            MOSEnum::Soar => "160",
        };
        let filename = format!("{}-{}.md", mos_code, slug);
        Ok(self.read_required(&self.root.join("mos").join(&filename))?)
    }

    fn protocol_template(&self, protocol_name: &str) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let filename = format!("{}.md", protocol_name.to_lowercase());
        Ok(self.read_required(&self.root.join("protocols").join(&filename))?)
    }

    fn assemble_task_prompt(
        &self,
        rank: &RankEnum,
        mos: &MOSEnum,
        task_description: &str,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let rank_md = self.rank_doctrine(rank)?;
        let mos_md = self.mos_doctrine(mos)?;
        Ok(format!(
            "You are a {} ({}) in the Rational Reserve with MOS {} ({}).\n\n\
             # Rank doctrine\n{}\n\n\
             # MOS doctrine\n{}\n\n\
             # Task\n{}\n\n\
             Execute this task according to your specialty and rank. \
             Provide a clear, actionable result.",
            rank.full_name(),
            rank.full_name(),
            mos.full_name(),
            mos.full_name(),
            rank_md,
            mos_md,
            task_description,
        ))
    }

    fn civilian_doctrine(
        &self,
        slug: &str,
    ) -> std::result::Result<CivilianDoctrine, Box<dyn std::error::Error + Send + Sync>> {
        // Reject slugs that could escape the doctrine root (e.g. "../secrets").
        validate_slug(slug)?;
        // Try flat first, then firm root. Each path component is a validated
        // slug or a compile-time literal — no caller-controlled traversal.
        let flat_path = self
            .root
            .join("civilian")
            .join("flat")
            .join(format!("{}.md", slug));
        let firm_path = self
            .root
            .join("civilian")
            .join("firms")
            .join(slug)
            .join("firm.md");

        let (path, raw) = if flat_path.is_file() {
            let body = self.read_required(&flat_path)?;
            (flat_path, body)
        } else if firm_path.is_file() {
            let body = self.read_required(&firm_path)?;
            (firm_path, body)
        } else {
            return Err(Box::new(DoctrineError::NotFound {
                path: format!(
                    "neither {} nor {} exists",
                    flat_path.display(),
                    firm_path.display()
                ),
            }));
        };

        parse_civilian_doctrine(&path, &raw).map_err(|e| e.into())
    }

    fn specialist_doctrine(
        &self,
        firm: &str,
        spec: &str,
    ) -> std::result::Result<CivilianDoctrine, Box<dyn std::error::Error + Send + Sync>> {
        // Both components are caller-supplied and must be validated before any
        // filesystem join (CWE-22).
        validate_slug(firm)?;
        validate_slug(spec)?;
        let path = self
            .root
            .join("civilian")
            .join("firms")
            .join(firm)
            .join(format!("specialist-{}.md", spec));
        if !path.is_file() {
            return Err(Box::new(DoctrineError::NotFound {
                path: path.display().to_string(),
            }));
        }
        let raw = self.read_required(&path)?;
        parse_civilian_doctrine(&path, &raw).map_err(|e| e.into())
    }
}

// ============================================================================
// Frontmatter parsing — minimal YAML for civilian doctrine files
// ============================================================================

/// Parse a markdown file with leading YAML-ish frontmatter into a
/// [`CivilianDoctrine`]. Supports scalar values and inline arrays only
/// (`key: value` or `key: [a, b, c]`); no nested maps, no block lists.
fn parse_civilian_doctrine(
    path: &PathBuf,
    raw: &str,
) -> std::result::Result<CivilianDoctrine, DoctrineError> {
    let raw = raw.trim_start_matches('\u{feff}'); // strip BOM if present
    let stripped = raw.strip_prefix("---\n").or_else(|| raw.strip_prefix("---\r\n"));
    let Some(after_open) = stripped else {
        return Err(DoctrineError::MalformedFrontmatter {
            path: path.display().to_string(),
            reason: "missing opening `---` delimiter".into(),
        });
    };

    let end = after_open
        .find("\n---\n")
        .or_else(|| after_open.find("\n---\r\n"))
        .ok_or_else(|| DoctrineError::MalformedFrontmatter {
            path: path.display().to_string(),
            reason: "missing closing `---` delimiter".into(),
        })?;

    let frontmatter_text = &after_open[..end];
    // Advance past the newline preceding the closing `---` and the closing line itself.
    let remainder = &after_open[end..];
    let body = remainder
        .strip_prefix("\n---\n")
        .or_else(|| remainder.strip_prefix("\n---\r\n"))
        .unwrap_or("");

    let mut slug = String::new();
    let mut kind_str: Option<String> = None;
    let mut firm: Option<String> = None;
    let mut project_path: Option<String> = None;
    let mut upstream_url: Option<String> = None;
    let mut languages: Vec<String> = Vec::new();
    let mut tier_override: Option<String> = None;
    let mut contract_scope: Vec<String> = Vec::new();
    let mut specialists: Vec<String> = Vec::new();

    for line in frontmatter_text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(DoctrineError::MalformedFrontmatter {
                path: path.display().to_string(),
                reason: format!("expected `key: value` in `{}`", line),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "slug" => slug = parse_scalar(value),
            "kind" => kind_str = Some(parse_scalar(value)),
            "firm" => firm = parse_optional(value),
            "project_path" => project_path = parse_optional(value),
            "upstream_url" => upstream_url = parse_optional(value),
            "languages" => languages = parse_array(value),
            "tier_override" => tier_override = parse_optional(value),
            "contract_scope" => contract_scope = parse_array(value),
            "specialists" => specialists = parse_array(value),
            _ => {} // ignore unknown keys for forward-compat
        }
    }

    if slug.is_empty() {
        return Err(DoctrineError::MalformedFrontmatter {
            path: path.display().to_string(),
            reason: "missing required `slug`".into(),
        });
    }
    let kind = match kind_str.as_deref() {
        Some("firm") => CivilianTier::Firm,
        Some("specialist") => CivilianTier::Specialist,
        Some("independent") | None => CivilianTier::Independent,
        Some(other) => {
            return Err(DoctrineError::MalformedFrontmatter {
                path: path.display().to_string(),
                reason: format!("unknown kind `{}`", other),
            });
        }
    };
    if kind == CivilianTier::Specialist && firm.is_none() {
        return Err(DoctrineError::MalformedFrontmatter {
            path: path.display().to_string(),
            reason: "`kind: specialist` requires `firm: <slug>`".into(),
        });
    }

    Ok(CivilianDoctrine {
        slug,
        kind,
        firm,
        project_path,
        upstream_url,
        languages,
        tier_override,
        contract_scope,
        specialists,
        body: body.to_string(),
    })
}

fn parse_scalar(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return stripped.to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return stripped.to_string();
    }
    trimmed.to_string()
}

fn parse_optional(value: &str) -> Option<String> {
    let s = parse_scalar(value);
    if s.is_empty() || s == "null" || s == "~" {
        None
    } else {
        Some(s)
    }
}

fn parse_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Vec::new();
    }
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(|s| parse_scalar(s))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Resolve the doctrine root path using the 3-tier strategy.
pub fn resolve_doctrine_root() -> Result<PathBuf> {
    // Tier 1: explicit env var
    if let Ok(env_path) = std::env::var("RR_DOCTRINE_PATH") {
        let candidate = PathBuf::from(&env_path);
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    // Tier 2: user-global runtime install
    let default = dirs::home_dir()
        .map(|h| h.join(".rational-reserve").join("doctrine"))
        .unwrap_or_default();
    if default.is_dir() {
        return Ok(default);
    }

    Err(DoctrineError::NoDoctrineRoot)
}

/// Build a default loader using the 3-tier resolution.
pub fn default_doctrine_loader() -> Result<FileSystemDoctrineLoader> {
    let root = resolve_doctrine_root()?;
    Ok(FileSystemDoctrineLoader::new(root))
}
