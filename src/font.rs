//! Font discovery & loading — ort-style.
//!
//! kou does not ship fonts. Instead, like [ort](https://crates.io/crates/ort)
//! for ONNX Runtime, it fetches a curated font family into a shared cache (once,
//! then cached across runs) and locates it transparently, so a consumer never
//! has to install fonts by hand. Sources are public (GitHub / jsDelivr CDN);
//! `KOU_FONT_MIRROR` and `KOU_DOWNLOAD_PROXY` route the fetch through a mirror
//! or forward proxy, which is essential behind restrictive networks.
//!
//! A [`FontSet`] pairs a primary monospace face (for Latin glyphs) with an
//! optional CJK fallback (思源黑体 / 更纱黑体 / 得意黑 / Source Han / Sarasa /
//! Smiley Sans). [`FontCache::load`] reads them into `ab_glyph` font vectors and
//! [`FontCache::glyph_for`] picks the first face that actually contains a given
//! codepoint — so a single render can mix Latin and CJK without tofu.
//!
//! Resolution order (mirrors shirabe's browser resolver):
//! 1. Explicit file paths — `KOU_FONT_PATH` (primary) and `KOU_FONT_CJK_PATH`.
//! 2. A previously-fetched copy in the shared cache.
//! 3. A runtime download (the `font-fetch` feature) from the registry.
//!
//! Knobs: `KOU_FONT_MIRROR`, `KOU_DOWNLOAD_PROXY`, `KOU_DOWNLOAD_TIMEOUT_SECS`,
//! `KOU_SKIP_FONT_FETCH`, `KOU_FONT_PRIMARY`, `KOU_FONT_CJK` (override the
//! registry families).

use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontVec, Glyph, PxScaleFont, ScaleFont};

/// A curated font family kou knows how to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    /// Fira Code — ligature-rich Latin monospace.
    FiraCode,
    /// JetBrains Mono.
    JetBrainsMono,
    /// Source Han Sans SC (思源黑体) — CJK sans serif.
    SourceHanSansSC,
    /// Sarasa Mono SC (更纱黑体) — CJK monospace, terminal-friendly.
    SarasaMonoSC,
    /// Smiley Sans (得意黑) — CJK display face.
    SmileySans,
}

impl FontFamily {
    /// The family selected for the primary (Latin) slot, or the default.
    pub fn primary_from_env() -> Self {
        match std::env::var("KOU_FONT_PRIMARY")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("jetbrains") | Some("jetbrainsmono") => FontFamily::JetBrainsMono,
            _ => FontFamily::FiraCode,
        }
    }

    /// The family selected for the CJK fallback slot; `None` disables CJK.
    pub fn cjk_from_env() -> Option<Self> {
        match std::env::var("KOU_FONT_CJK")
            .ok()
            .map(|s| s.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("sourcehansans") | Some("sourcehan") => Some(FontFamily::SourceHanSansSC),
            Some("sarasa") | Some("sarasa-mono") => Some(FontFamily::SarasaMonoSC),
            Some("smileysans") | Some("smiley") => Some(FontFamily::SmileySans),
            Some("none") | Some("off") => None,
            _ => Some(FontFamily::SarasaMonoSC),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FontFamily::FiraCode => "fira-code",
            FontFamily::JetBrainsMono => "jetbrains-mono",
            FontFamily::SourceHanSansSC => "source-han-sans-sc",
            FontFamily::SarasaMonoSC => "sarasa-mono-sc",
            FontFamily::SmileySans => "smiley-sans",
        }
    }

    /// Filename inside the cache.
    fn cache_name(self) -> &'static str {
        match self {
            FontFamily::FiraCode => "FiraCode-Regular.ttf",
            FontFamily::JetBrainsMono => "JetBrainsMono-Regular.ttf",
            FontFamily::SourceHanSansSC => "SourceHanSansSC-Regular.otf",
            FontFamily::SarasaMonoSC => "SarasaMonoSC-Regular.ttf",
            FontFamily::SmileySans => "SmileySans-Regular.ttf",
        }
    }

    /// Source URL. `KOU_FONT_MIRROR` substitutes the canonical GitHub/jsDelivr
    /// host so a mirror or GFW-friendly host can serve the bytes instead.
    fn source_url(self) -> String {
        let raw = match self {
            FontFamily::FiraCode => {
                "https://cdn.jsdelivr.net/gh/tonsky/FiraCode@5.2/distr/ttf/FiraCode-Regular.ttf"
            }
            FontFamily::JetBrainsMono => {
                "https://cdn.jsdelivr.net/gh/JetBrains/JetBrainsMono@2.304/fonts/ttf/JetBrainsMono-Regular.ttf"
            }
            FontFamily::SourceHanSansSC => {
                "https://cdn.jsdelivr.net/gh/adobe-fonts/source-han-sans@release/SubsetOTF/CN/SourceHanSansCN-Regular.otf"
            }
            FontFamily::SarasaMonoSC => {
                "https://cdn.jsdelivr.net/gh/be5invis/sarasa-gothic@0.42.5/SarasaMonoSC-Regular.ttf"
            }
            FontFamily::SmileySans => {
                "https://cdn.jsdelivr.net/gh/atelier-anchor/smiley-sans@2.0.1/fonts/SmileySans-Regular.ttf"
            }
        };
        if let Ok(mirror) = std::env::var("KOU_FONT_MIRROR") {
            let mirror = mirror.trim_end_matches('/');
            if !mirror.is_empty() {
                return raw
                    .replace("https://cdn.jsdelivr.net/gh", mirror)
                    .replace("https://github.com", mirror);
            }
        }
        raw.to_string()
    }
}

/// A primary face plus an optional CJK fallback.
#[derive(Debug, Clone)]
pub struct FontSet {
    pub primary: FontFamily,
    pub cjk: Option<FontFamily>,
}

impl FontSet {
    /// The default set, honouring `KOU_FONT_PRIMARY` / `KOU_FONT_CJK`.
    pub fn from_env() -> Self {
        FontSet {
            primary: FontFamily::primary_from_env(),
            cjk: FontFamily::cjk_from_env(),
        }
    }
}

/// Shared cache root: `<cache>/kou/fonts`.
pub fn cache_root() -> PathBuf {
    cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("kou-cache"))
        .join("kou")
        .join("fonts")
}

fn cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        None
    }
}

/// Resolve a single family to an existing font file, fetching if necessary.
///
/// Returns `None` when the `font-fetch` feature is disabled and no cached copy
/// or explicit path exists, so callers can fall back to a built-in font.
pub fn resolve_family(family: FontFamily) -> Option<PathBuf> {
    // 1. Explicit override (only meaningful for the primary slot, but cheap).
    if family == FontFamily::primary_from_env() {
        if let Ok(p) = std::env::var("KOU_FONT_PATH") {
            let path = PathBuf::from(&p);
            if path.exists() {
                return Some(path);
            }
        }
    }
    if family == FontFamily::SarasaMonoSC
        || matches!(family, FontFamily::SourceHanSansSC | FontFamily::SmileySans)
    {
        if let Ok(p) = std::env::var("KOU_FONT_CJK_PATH") {
            let path = PathBuf::from(&p);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // 2. Cached copy.
    let cached = cache_root().join(family.cache_name());
    if cached.exists() {
        return Some(cached);
    }

    // 3. Runtime fetch.
    #[cfg(feature = "font-fetch")]
    {
        if std::env::var_os("KOU_SKIP_FONT_FETCH").is_none() {
            match download(family, &cached) {
                Ok(()) => return Some(cached),
                Err(e) => {
                    eprintln!("[kou] font fetch for {} failed: {e}", family.label());
                }
            }
        }
    }

    None
}

#[cfg(feature = "font-fetch")]
fn download(family: FontFamily, dest: &Path) -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let timeout = std::env::var("KOU_DOWNLOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &u64| n > 0)
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(120));
    let mut builder = reqwest::blocking::Client::builder().timeout(timeout);
    if let Ok(proxy) = std::env::var("KOU_DOWNLOAD_PROXY") {
        let proxy = proxy.trim();
        if !proxy.is_empty() {
            eprintln!("[kou] using font download proxy {proxy}");
            builder = builder.proxy(
                reqwest::Proxy::all(proxy)
                    .map_err(|e| anyhow::anyhow!("invalid KOU_DOWNLOAD_PROXY {proxy:?}: {e}"))?,
            );
        }
    }
    let client = builder.build()?;
    let url = family.source_url();
    eprintln!("[kou] downloading {} (once, then cached)", url);
    let bytes = client
        .get(&url)
        .header("User-Agent", "kou")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write to a temp file first, then rename, so a partial download never
    // poisons the cache.
    let tmp = dest.with_extension("download");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

/// In-memory, rasterisation-ready font faces.
pub struct FontCache {
    /// Primary first, then CJK fallback. Ordered so [`glyph_for`] picks Latin
    /// before CJK when both cover a codepoint.
    faces: Vec<PxScaleFont<FontVec>>,
}

impl FontCache {
    /// Load a [`FontSet`] from disk into `ab_glyph`. Faces that fail to parse
    /// are skipped (with a warning), so a missing CJK face degrades gracefully.
    pub fn load(set: &FontSet, px: f32) -> Self {
        let mut faces = Vec::new();
        if let Some(path) = resolve_family(set.primary) {
            push_face(&mut faces, &path, px);
        }
        if let Some(cjk) = set.cjk {
            if let Some(path) = resolve_family(cjk) {
                push_face(&mut faces, &path, px);
            }
        }
        FontCache { faces }
    }

    /// Build a cache from explicit file paths (no fetching).
    pub fn from_paths(paths: &[&Path], px: f32) -> Self {
        let mut faces = Vec::new();
        for p in paths {
            push_face(&mut faces, p, px);
        }
        FontCache { faces }
    }

    /// Number of usable faces.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// `true` if no face loaded.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// An empty cache (no faces) — the renderer degrades to solid blocks.
    pub fn empty() -> Self {
        FontCache { faces: Vec::new() }
    }

    /// Pick the first face that defines a glyph for `ch`, returning a scaled
    /// glyph ready to outline. `ab_glyph` reports a missing glyph as
    /// `GlyphId(0)`, which we treat as "not in this face".
    pub fn glyph_for(&self, ch: char) -> Option<Glyph> {
        for face in &self.faces {
            let glyph_id = face.glyph_id(ch);
            if glyph_id.0 != 0 {
                return Some(face.scaled_glyph(ch));
            }
        }
        None
    }

    /// Rasterise `ch` into a per-pixel coverage callback, using the first face
    /// that covers the codepoint (Latin before CJK). `cell_x`/`cell_y` is the
    /// cell's top-left in pixels; the ascent offset is applied internally. The
    /// callback receives **absolute** pixel coordinates (already offset by the
    /// glyph's positioned bounding box), so the caller must NOT add the cell
    /// origin again. Returns `true` if any face rendered the glyph.
    pub fn draw_char<F: FnMut(u32, u32, f32)>(
        &self,
        ch: char,
        cell_x: f32,
        cell_y: f32,
        mut put: F,
    ) -> bool {
        for face in &self.faces {
            if face.glyph_id(ch).0 == 0 {
                continue;
            }
            let mut glyph = face.scaled_glyph(ch);
            glyph.position = ab_glyph::point(cell_x, cell_y + face.ascent());
            let Some(outlined) = face.outline_glyph(glyph) else {
                continue;
            };
            // `OutlinedGlyph::draw` yields coords relative to the glyph's
            // bounding-box min; offset them back to absolute image space.
            let bounds = outlined.px_bounds();
            let min_x = bounds.min.x.floor();
            let min_y = bounds.min.y.floor();
            outlined.draw(|gx, gy, cov| {
                let x = min_x + gx as f32;
                let y = min_y + gy as f32;
                if x >= 0.0 && y >= 0.0 {
                    put(x as u32, y as u32, cov);
                }
            });
            return true;
        }
        false
    }

    /// Borrow the primary (first) face — used for metrics like line height.
    pub fn primary(&self) -> Option<&PxScaleFont<FontVec>> {
        self.faces.first()
    }
}

fn push_face(out: &mut Vec<PxScaleFont<FontVec>>, path: &Path, px: f32) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[kou] cannot read font {}: {e}", path.display());
            return;
        }
    };
    match FontVec::try_from_vec(bytes) {
        // `into_scaled` consumes the font and yields an owned
        // `PxScaleFont<'static, FontVec>` — no leaking needed.
        Ok(font) => out.push(font.into_scaled(px)),
        Err(e) => {
            eprintln!("[kou] font {} failed to parse: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_name_is_unique_per_family() {
        let names: Vec<&str> = [
            FontFamily::FiraCode,
            FontFamily::JetBrainsMono,
            FontFamily::SourceHanSansSC,
            FontFamily::SarasaMonoSC,
            FontFamily::SmileySans,
        ]
        .iter()
        .map(|f| f.cache_name())
        .collect();
        let dedup: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), dedup.len(), "duplicate cache names: {names:?}");
    }

    #[test]
    fn mirror_substitution_replaces_host() {
        let restore = std::env::var_os("KOU_FONT_MIRROR");
        // SAFETY: serial env mutation in a single-threaded test.
        unsafe { std::env::set_var("KOU_FONT_MIRROR", "https://mirror.example.com") };
        let url = FontFamily::FiraCode.source_url();
        assert!(
            url.starts_with("https://mirror.example.com/"),
            "mirror not applied: {url}"
        );
        unsafe {
            match restore {
                Some(v) => std::env::set_var("KOU_FONT_MIRROR", v),
                None => std::env::remove_var("KOU_FONT_MIRROR"),
            }
        }
    }
}
