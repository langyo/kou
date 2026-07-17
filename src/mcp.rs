//! Standalone MCP (Model Context Protocol) server for kou.
//!
//! Exposes the virtual-terminal engine — PTY sessions, VT100 screen,
//! key/text input, screenshots and PNG rendering — as MCP tools over stdio,
//! so an AI coding assistant can drive real terminals the same way the
//! tairitsu packager's vtty tools did, but with no browser/daemon dependency.
//!
//! This is the kou half of what `tairitsu-mcp` shipped; the browser half lives
//! in shirabe. Activate with the `mcp` cargo feature and `kou mcp`.
//!
//! # Usage
//!
//! ```ignore
//! kou mcp
//! ```

#![cfg(feature = "mcp")]

use anyhow::Result;
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters, model::*, service::RequestContext, tool, tool_handler,
    tool_router,
};
use schemars::JsonSchema;

/// Font size used when rasterising VTty screenshots to PNG.
///
/// The renderer expects fonts loaded at `font_px * supersample`.
const FONT_PX: f32 = 32.0;
/// Supersample factor — render at this multiple then downscale for crisp glyphs.
const RENDER_SUPER: u32 = 3;

struct Server {
    vtty: crate::VttyManager,
    fonts: Arc<crate::FontCache>,
    cwd: Arc<RwLock<Option<String>>>,
}

impl Server {
    fn tool_result(text: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(text)])
    }

    /// Rasterise a VTty screen to a base64-encoded PNG, painted through `theme`.
    fn render_png(&self, screen: &crate::Screen, theme: &crate::Theme) -> Result<String, McpError> {
        let png = crate::render::render_png_supersampled(
            screen,
            &self.fonts,
            FONT_PX,
            RENDER_SUPER,
            theme,
        )
        .map_err(|e| McpError::internal_error(format!("VTty render failed: {e}"), None))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&png))
    }

    /// Build the JSON object the `text` / `both` screenshot modes emit.
    fn screen_text_json(
        &self,
        session_id: &str,
        alive: bool,
        screen: &crate::Screen,
        text: &str,
    ) -> String {
        json!({
            "session_id": session_id,
            "alive": alive,
            "rows": screen.rows,
            "cols": screen.cols,
            "text": text,
        })
        .to_string()
    }
}

// ── Tool argument structs ────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct VttyLaunchArgs {
    command: String,
    cols: Option<u64>,
    rows: Option<u64>,
    env: Option<String>,
    cwd: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttySessionArgs {
    session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttyScreenshotArgs {
    session_id: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    theme: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttySendKeysArgs {
    session_id: String,
    keys: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttySendTextArgs {
    session_id: String,
    text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttyWaitArgs {
    session_id: String,
    seconds: Option<f64>,
    pattern: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttyReadyArgs {
    session_id: String,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VttyResizeArgs {
    session_id: String,
    cols: u64,
    rows: u64,
}

// ── VTty tools (in-process, via kou::VttyManager) ────────────────────

#[tool_router]
impl Server {
    #[tool(description = "Launch a command in a virtual terminal session")]
    async fn vtty_launch(
        &self,
        Parameters(args): Parameters<VttyLaunchArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let resolved_cwd = match args.cwd.as_deref() {
            Some(c) => Some(c.to_string()),
            None => resolve_default_cwd(&context, &self.cwd).await,
        };
        let env_pairs = parse_env_string(args.env.as_deref().unwrap_or(""));
        let env_refs: Vec<(&str, &str)> = env_pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let info = self
            .vtty
            .launch(
                &args.command,
                resolved_cwd.as_deref(),
                &env_refs,
                args.cols.unwrap_or(120) as u16,
                args.rows.unwrap_or(40) as u16,
                args.name.as_deref(),
            )
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(Self::tool_result(
            serde_json::to_string_pretty(&info).unwrap_or_default(),
        ))
    }

    #[tool(description = "Kill a virtual terminal session")]
    async fn vtty_kill(
        &self,
        Parameters(args): Parameters<VttySessionArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let info = self.vtty.kill(&args.session_id).await.ok_or_else(|| {
            McpError::internal_error(format!("Session '{}' not found", args.session_id), None)
        })?;
        Ok(Self::tool_result(
            serde_json::to_string_pretty(&info).unwrap_or_default(),
        ))
    }

    #[tool(
        description = "Send key sequences to a virtual terminal. Supports Enter, Tab, Escape, Backspace, Delete, Arrow keys, Home/End, PageUp/PageDown, F1-F12, Ctrl+X, Alt+X"
    )]
    async fn vtty_send_keys(
        &self,
        Parameters(args): Parameters<VttySendKeysArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.vtty
            .send_keys(&args.session_id, &args.keys)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(Self::tool_result(
            json!({"session_id": args.session_id, "keys": args.keys, "sent": true}).to_string(),
        ))
    }

    #[tool(description = "Send text string to a virtual terminal")]
    async fn vtty_send_text(
        &self,
        Parameters(args): Parameters<VttySendTextArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.vtty
            .send_text(&args.session_id, &args.text)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(Self::tool_result(
            json!({"session_id": args.session_id, "length": args.text.len(), "sent": true})
                .to_string(),
        ))
    }

    #[tool(
        description = "Capture current terminal screen content as text (text-only models) and/or as a rendered PNG image (vision-capable models). \
        The 'format' parameter controls output: 'text' (default) returns plain text, 'image' returns a rendered PNG, 'both' returns both. \
        The 'theme' parameter selects the PNG colour scheme (Windows Terminal schemes): campbell (default), campbell-powershell, vintage, one-half-dark, one-half-light, solarized-dark, solarized-light, tango-dark, tango-light, dimidium, ottosson, dark+, cga, ibm-5153, xterm. Unknown names fall back to campbell."
    )]
    async fn vtty_screenshot(
        &self,
        Parameters(args): Parameters<VttyScreenshotArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let fmt = args.format.as_deref().unwrap_or("text");
        let theme = crate::theme_by_name(args.theme.as_deref().unwrap_or("campbell"));

        let screen = self
            .vtty
            .screen(&args.session_id)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        let alive = self
            .vtty
            .ping(&args.session_id)
            .await
            .map(|i| i.alive)
            .unwrap_or(false);
        let text = screen.text();

        match fmt {
            "image" => {
                let b64 = self.render_png(&screen, theme)?;
                Ok(CallToolResult::success(vec![ContentBlock::image(
                    b64,
                    "image/png",
                )]))
            }
            "both" => {
                let b64 = self.render_png(&screen, theme)?;
                Ok(CallToolResult::success(vec![
                    ContentBlock::text(self.screen_text_json(
                        &args.session_id,
                        alive,
                        &screen,
                        &text,
                    )),
                    ContentBlock::image(b64, "image/png"),
                ]))
            }
            _ => Ok(Self::tool_result(self.screen_text_json(
                &args.session_id,
                alive,
                &screen,
                &text,
            ))),
        }
    }

    #[tool(description = "Wait for duration or until text appears on screen")]
    async fn vtty_wait(
        &self,
        Parameters(args): Parameters<VttyWaitArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let secs = args.seconds.unwrap_or(5.0);
        let pattern = args.pattern.unwrap_or_default();
        if !pattern.is_empty() {
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_secs_f64(secs.min(1800.0));
            let mut found = false;
            while std::time::Instant::now() < deadline {
                let alive = self
                    .vtty
                    .ping(&args.session_id)
                    .await
                    .map(|i| i.alive)
                    .unwrap_or(false);
                if !alive {
                    break;
                }
                let hits = self
                    .vtty
                    .find_text(&args.session_id, &pattern)
                    .await
                    .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
                if !hits.is_empty() {
                    found = true;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
            let alive = self
                .vtty
                .ping(&args.session_id)
                .await
                .map(|i| i.alive)
                .unwrap_or(false);
            Ok(Self::tool_result(
                json!({"session_id": args.session_id, "pattern": pattern, "found": found, "alive": alive})
                    .to_string(),
            ))
        } else {
            let wait_secs = secs.min(1800.0) as u64;
            let mut alive = true;
            for _ in 0..(wait_secs * 20) {
                alive = self
                    .vtty
                    .ping(&args.session_id)
                    .await
                    .map(|i| i.alive)
                    .unwrap_or(false);
                if !alive {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Ok(Self::tool_result(
                json!({"session_id": args.session_id, "seconds_waited": secs, "alive": alive})
                    .to_string(),
            ))
        }
    }

    #[tool(
        description = "Wait until a VTty session has screen output (useful after vtty_launch for slow-starting commands). Returns immediately if output is already present."
    )]
    async fn vtty_ready(
        &self,
        Parameters(args): Parameters<VttyReadyArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let timeout_ms = args.timeout_ms.unwrap_or(30000);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut ready = false;
        while std::time::Instant::now() < deadline {
            let has = self
                .vtty
                .has_output(&args.session_id)
                .await
                .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
            if has {
                ready = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Ok(Self::tool_result(
            json!({"session_id": args.session_id, "ready": ready}).to_string(),
        ))
    }

    #[tool(
        description = "Get the scrollback buffer (history) of a virtual terminal session, including current screen content"
    )]
    async fn vtty_scrollback(
        &self,
        Parameters(args): Parameters<VttySessionArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let text = self
            .vtty
            .scrollback(&args.session_id)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(Self::tool_result(
            json!({"session_id": args.session_id, "text": text}).to_string(),
        ))
    }

    #[tool(description = "Resize a virtual terminal")]
    async fn vtty_resize(
        &self,
        Parameters(args): Parameters<VttyResizeArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let old = self
            .vtty
            .ping(&args.session_id)
            .await
            .map(|i| (i.cols, i.rows));
        self.vtty
            .resize(&args.session_id, args.cols as u16, args.rows as u16)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(Self::tool_result(
            json!({"session_id": args.session_id, "old": old.map(|(c,r)| json!({"cols": c, "rows": r})), "new": {"cols": args.cols, "rows": args.rows}})
                .to_string(),
        ))
    }

    #[tool(description = "List all active virtual terminal sessions")]
    async fn vtty_list(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let sessions = self.vtty.list().await;
        Ok(Self::tool_result(
            serde_json::to_string_pretty(&sessions).unwrap_or_else(|_| "[]".to_string()),
        ))
    }

    #[tool(
        description = "Check if a VTty session's child process is still alive and refresh screen state"
    )]
    async fn vtty_ping(
        &self,
        Parameters(args): Parameters<VttySessionArgs>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let info = self.vtty.ping(&args.session_id).await.ok_or_else(|| {
            McpError::internal_error(format!("Session '{}' not found", args.session_id), None)
        })?;
        Ok(Self::tool_result(
            serde_json::to_string_pretty(&info).unwrap_or_default(),
        ))
    }
}

// ── ServerHandler ────────────────────────────────────

#[tool_handler(router = Server::tool_router())]
impl ServerHandler for Server {}

// ── helpers ──────────────────────────────────────────

/// Parse an env-string of the form `"K=V\nK2=V2"` (newlines or commas) into
/// owned pairs. Malformed entries are dropped.
fn parse_env_string(raw: &str) -> Vec<(String, String)> {
    raw.split([',', '\n'])
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            let (k, v) = pair.split_once('=')?;
            Some((k.trim().to_string(), v.to_string()))
        })
        .collect()
}

/// Resolve the working directory for a launched session, in priority order:
/// an explicit `KOU_PROJECT_ROOT` env var → the client's `roots` capability
/// (if advertised) → this process's current dir.
async fn resolve_default_cwd(
    context: &RequestContext<RoleServer>,
    cached: &Arc<RwLock<Option<String>>>,
) -> Option<String> {
    if let Ok(root) = std::env::var("KOU_PROJECT_ROOT") {
        if !root.is_empty() {
            return Some(root);
        }
    }

    if let Some(info) = context.peer.peer_info() {
        if info.capabilities.roots.is_some() {
            // `list_roots` is deprecated by SEP-2577 but remains the only way to
            // honor a client-advertised project root today; keep using it until a
            // replacement ships.
            #[allow(deprecated)]
            if let Ok(result) = context.peer.list_roots().await {
                if let Some(root) = result.roots.first() {
                    let uri = &root.uri;
                    let path = if let Some(p) = uri.strip_prefix("file://") {
                        p.to_string()
                    } else if let Some(p) = uri.strip_prefix("file:") {
                        p.to_string()
                    } else {
                        uri.clone()
                    };
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
        }
    }

    if let Some(c) = cached.read().await.clone() {
        return Some(c);
    }

    if let Ok(cwd) = std::env::current_dir() {
        return Some(cwd.to_string_lossy().to_string());
    }

    None
}

// ── public entry point ───────────────────────────────

pub async fn run() -> Result<()> {
    // Load VTty fonts once. Strategy: system fonts first (fast, zero-network,
    // includes CJK if NotoSansCJK is installed), then async fetch as fallback.
    // Fonts loaded at supersampled resolution (font_px × supersample).
    let font_px = FONT_PX * RENDER_SUPER as f32;
    let fonts = {
        let sys = crate::FontCache::from_system_fonts(font_px);
        if !sys.is_empty() {
            sys
        } else {
            let font_set = crate::FontSet::from_env();
            let remote = crate::FontCache::load_async(&font_set, font_px).await;
            if remote.is_empty() {
                crate::FontCache::empty()
            } else {
                remote
            }
        }
    };

    let server = Server {
        vtty: crate::VttyManager::new(),
        fonts: Arc::new(fonts),
        cwd: Arc::new(RwLock::new(
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string()),
        )),
    };

    let transport = rmcp::transport::stdio();
    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;

    Ok(())
}
