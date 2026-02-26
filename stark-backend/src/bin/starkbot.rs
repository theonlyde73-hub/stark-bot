//! StarkBot Interactive CLI
//!
//! An interactive terminal client for StarkBot with chat REPL and module TUI support.
//!
//! ## Usage
//!
//!   EXT_CHANNEL_API_TOKEN="your-token" cargo run --bin starkbot
//!
//! ## Environment variables
//!
//!   EXT_CHANNEL_API_TOKEN - External channel API token (required)
//!   EXT_CHANNEL_URL       - Gateway base URL (default: http://localhost:8080)

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead, Write};

// ── Config ──────────────────────────────────────────────────────────────

struct Config {
    base_url: String,
    token: String,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let token = env::var("EXT_CHANNEL_API_TOKEN")
            .map_err(|_| "EXT_CHANNEL_API_TOKEN is required".to_string())?;
        let base_url = env::var("EXT_CHANNEL_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        Ok(Self { base_url, token })
    }
}

// ── SSE helpers ─────────────────────────────────────────────────────────

/// Parse SSE events from a byte stream, calling `on_event` for each complete event.
/// Returns when the stream ends or on_event returns false.
async fn consume_sse<F>(response: reqwest::Response, mut on_event: F)
where
    F: FnMut(&str, &str) -> bool,
{
    let mut bytes_stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = bytes_stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(_) => break,
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_str = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let mut event_name = "";
            let mut data_lines = Vec::new();

            for line in event_str.lines() {
                if let Some(ev) = line.strip_prefix("event: ") {
                    event_name = ev.trim();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    data_lines.push(d);
                }
            }

            let data = data_lines.join("\n");
            if !on_event(event_name, &data) {
                return;
            }
        }
    }
}

// ── Chat REPL ───────────────────────────────────────────────────────────

async fn cmd_help() {
    println!("Commands:");
    println!("  /modules list           - List installed modules");
    println!("  /modules connect <name> - Connect to module TUI");
    println!("  /session new            - Create a new chat session");
    println!("  /help                   - Show this help");
    println!("  /quit                   - Exit");
    println!();
    println!("Any other text is sent as a chat message (streamed response).");
}

async fn cmd_modules_list(client: &Client, cfg: &Config) {
    let url = format!("{}/api/gateway/modules", cfg.base_url);
    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("Error: HTTP {}", resp.status());
        return;
    }

    let body: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing response: {}", e);
            return;
        }
    };

    if let Some(modules) = body.get("modules").and_then(|v| v.as_array()) {
        if modules.is_empty() {
            println!("No modules installed.");
            return;
        }
        println!(
            "{:<20} {:<10} {:<5} {}",
            "NAME", "VERSION", "TUI", "DESCRIPTION"
        );
        println!("{}", "-".repeat(70));
        for m in modules {
            let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let version = m.get("version").and_then(|v| v.as_str()).unwrap_or("?");
            let has_tui = m
                .get("has_tui")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let desc = m
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!(
                "{:<20} {:<10} {:<5} {}",
                name,
                version,
                if has_tui { "yes" } else { "no" },
                desc
            );
        }
    }
}

async fn cmd_session_new(client: &Client, cfg: &Config) {
    let url = format!("{}/api/gateway/sessions/new", cfg.base_url);
    let resp = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let body: Value = resp.json().await.unwrap_or_default();
    if let Some(sid) = body.get("session_id").and_then(|v| v.as_i64()) {
        println!("New session created: {}", sid);
    } else {
        eprintln!(
            "Error: {}",
            body.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
        );
    }
}

async fn cmd_chat_stream(client: &Client, cfg: &Config, message: &str) {
    let url = format!("{}/api/gateway/chat/stream", cfg.base_url);
    let body = json!({ "message": message });

    let resp = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("Error: HTTP {}", resp.status());
        return;
    }

    consume_sse(resp, |_event_name, data| {
        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
            let event_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match event_type {
                "text" => {
                    let content = parsed
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    print!("{}", content);
                    let _ = io::stdout().flush();
                }
                "tool_call" => {
                    let tool = parsed
                        .get("tool_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    eprintln!("  [tool] {}", tool);
                }
                "tool_result" => {
                    let tool = parsed
                        .get("tool_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let ok = parsed
                        .get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    eprintln!(
                        "  [result] {} {}",
                        tool,
                        if ok { "ok" } else { "failed" }
                    );
                }
                "thinking" => {
                    let msg = parsed
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("thinking...");
                    eprintln!("  [thinking] {}", msg);
                }
                "subagent_spawned" => {
                    let label = parsed
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    eprintln!("  [subagent] spawned: {}", label);
                }
                "subagent_completed" => {
                    let label = parsed
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    eprintln!("  [subagent] completed: {}", label);
                }
                "done" => {
                    println!();
                    return false;
                }
                _ => {}
            }
        }
        true
    })
    .await;
}

// ── TUI Mode ────────────────────────────────────────────────────────────

struct TuiState {
    module_name: String,
    selected: i64,
    scroll: i64,
    actions: Vec<Value>,
    last_frame: String,
    status_msg: String,
    confirming: Option<Value>,
}

impl TuiState {
    fn new(module_name: String) -> Self {
        Self {
            module_name,
            selected: 0,
            scroll: 0,
            actions: Vec::new(),
            last_frame: String::new(),
            status_msg: String::new(),
            confirming: None,
        }
    }

    fn find_action_by_key(&self, key: char) -> Option<&Value> {
        self.actions.iter().find(|a| {
            a.get("key")
                .and_then(|v| v.as_str())
                .and_then(|s| s.chars().next())
                == Some(key)
        })
    }
}

/// Fetch a TUI frame directly (not SSE, just the frame content via the SSE endpoint
/// with short-lived connection, or direct module endpoint)
async fn fetch_tui_frame_direct(
    client: &Client,
    cfg: &Config,
    state: &TuiState,
) -> Result<(String, Vec<Value>), String> {
    let (w, h) = terminal::size().unwrap_or((120, 40));
    // Use the SSE stream endpoint but we only take the first frame
    let url = format!(
        "{}/api/gateway/modules/{}/tui/stream?width={}&height={}&selected={}&scroll={}",
        cfg.base_url,
        state.module_name,
        w,
        h.saturating_sub(2), // leave room for status line
        state.selected,
        state.scroll
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let mut frame = String::new();
    let mut actions = Vec::new();

    consume_sse(resp, |event_name, data| {
        match event_name {
            "tui_frame" => {
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    if let Some(ansi) = parsed.get("ansi").and_then(|v| v.as_str()) {
                        frame = ansi.to_string();
                    }
                    if let Some(acts) = parsed.get("actions").and_then(|v| v.as_array()) {
                        actions = acts.clone();
                    }
                }
                // Got our frame, stop consuming
                false
            }
            "error" => {
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    let err = parsed
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    frame = format!("Error: {}", err);
                }
                false
            }
            _ => true,
        }
    })
    .await;

    Ok((frame, actions))
}

/// Post a TUI action to the module
async fn post_tui_action(
    client: &Client,
    cfg: &Config,
    state: &TuiState,
    action_name: &str,
    inputs: Option<Value>,
) -> Result<Value, String> {
    let url = format!(
        "{}/api/gateway/modules/{}/tui/action",
        cfg.base_url, state.module_name
    );

    let mut body = json!({
        "action": action_name,
        "state": {
            "selected": state.selected,
            "scroll": state.scroll,
        }
    });
    if let Some(inp) = inputs {
        body["inputs"] = inp;
    }

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    resp.json::<Value>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

/// Render the TUI frame + status bar to the terminal
fn render_tui(state: &TuiState) {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All));
    print!("{}", state.last_frame);

    // Status line at bottom
    let (w, h) = terminal::size().unwrap_or((120, 40));
    let _ = execute!(stdout, cursor::MoveTo(0, h.saturating_sub(1)));

    let status = if let Some(ref action) = state.confirming {
        let name = action
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("action");
        format!(
            "\x1b[7m Confirm '{}' on row {}? [y/n] \x1b[0m",
            name, state.selected
        )
    } else if !state.status_msg.is_empty() {
        format!("\x1b[7m {} \x1b[0m", state.status_msg)
    } else {
        // Build hotkey hints from actions
        let hints: Vec<String> = state
            .actions
            .iter()
            .filter_map(|a| {
                let key = a.get("key").and_then(|v| v.as_str())?;
                let name = a.get("name").and_then(|v| v.as_str())?;
                Some(format!("[{}]{}", key, name))
            })
            .collect();
        let action_hints = if hints.is_empty() {
            String::new()
        } else {
            format!(" | {}", hints.join(" "))
        };
        format!(
            "\x1b[7m [{} sel={} scroll={}] q:quit Up/Down:nav PgUp/PgDn:jump{} \x1b[0m",
            state.module_name, state.selected, state.scroll, action_hints
        )
    };

    // Truncate status to terminal width
    let status_display: String = status.chars().take(w as usize).collect();
    print!("{}", status_display);
    let _ = stdout.flush();
}

/// Read multiple prompted inputs (exits raw mode temporarily)
fn read_prompted_inputs(fields: &[&str]) -> Option<Value> {
    let mut stdout = io::stdout();
    let _ = terminal::disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen);

    let mut inputs = serde_json::Map::new();
    let stdin = io::stdin();

    for field in fields {
        print!("{}: ", field);
        let _ = stdout.flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            // Re-enter TUI
            let _ = execute!(stdout, EnterAlternateScreen);
            let _ = terminal::enable_raw_mode();
            return None;
        }
        let val = line.trim().to_string();
        if val.is_empty() {
            // Re-enter TUI
            let _ = execute!(stdout, EnterAlternateScreen);
            let _ = terminal::enable_raw_mode();
            return None;
        }
        inputs.insert(field.to_string(), Value::String(val));
    }

    let _ = execute!(stdout, EnterAlternateScreen);
    let _ = terminal::enable_raw_mode();
    Some(Value::Object(inputs))
}

/// Main TUI mode loop
async fn run_tui_mode(client: &Client, cfg: &Config, module_name: &str) {
    let mut tui = TuiState::new(module_name.to_string());
    let mut stdout = io::stdout();

    // Enter raw mode + alternate screen
    if terminal::enable_raw_mode().is_err() {
        eprintln!("Error: Could not enable raw mode");
        return;
    }
    if execute!(stdout, EnterAlternateScreen, cursor::Hide).is_err() {
        let _ = terminal::disable_raw_mode();
        eprintln!("Error: Could not enter alternate screen");
        return;
    }

    // Initial frame fetch
    match fetch_tui_frame_direct(client, cfg, &tui).await {
        Ok((frame, actions)) => {
            tui.last_frame = frame;
            tui.actions = actions;
        }
        Err(e) => {
            tui.status_msg = format!("Error: {}", e);
        }
    }
    render_tui(&tui);

    // Start background SSE listener for push updates
    let sse_url = {
        let (w, h) = terminal::size().unwrap_or((120, 40));
        format!(
            "{}/api/gateway/modules/{}/tui/stream?width={}&height={}",
            cfg.base_url,
            module_name,
            w,
            h.saturating_sub(2)
        )
    };
    let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel::<(String, Vec<Value>)>(8);
    let sse_client = client.clone();
    let sse_token = cfg.token.clone();
    let sse_handle = tokio::spawn(async move {
        let resp = match sse_client
            .get(&sse_url)
            .header("Authorization", format!("Bearer {}", sse_token))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => return,
        };

        // Skip the first frame (we already fetched it directly)
        let mut skip_first = true;
        consume_sse(resp, |event_name, data| {
            if event_name == "tui_frame" {
                if skip_first {
                    skip_first = false;
                    return true;
                }
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    let frame = parsed
                        .get("ansi")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let actions = parsed
                        .get("actions")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    // Best effort send; if receiver is gone, stop
                    if sse_tx.blocking_send((frame, actions)).is_err() {
                        return false;
                    }
                }
            }
            true
        })
        .await;
    });

    // Main event loop
    loop {
        // Check for pushed SSE frames (non-blocking)
        while let Ok((frame, actions)) = sse_rx.try_recv() {
            tui.last_frame = frame;
            if !actions.is_empty() {
                tui.actions = actions;
            }
            render_tui(&tui);
        }

        // Poll for keyboard events (100ms timeout so we can check SSE)
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                // Handle confirmation mode
                if tui.confirming.is_some() {
                    let action = tui.confirming.take().unwrap();
                    if key.code == KeyCode::Char('y') || key.code == KeyCode::Char('Y') {
                        let action_name = action
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Check if action needs input fields
                        let input_fields: Vec<String> = action
                            .get("inputs")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let inputs = if input_fields.is_empty() {
                            None
                        } else {
                            let field_refs: Vec<&str> =
                                input_fields.iter().map(|s| s.as_str()).collect();
                            match read_prompted_inputs(&field_refs) {
                                Some(v) => Some(v),
                                None => {
                                    tui.status_msg = "Cancelled.".to_string();
                                    render_tui(&tui);
                                    continue;
                                }
                            }
                        };

                        tui.status_msg = format!("Executing '{}'...", action_name);
                        render_tui(&tui);

                        match post_tui_action(client, cfg, &tui, action_name, inputs).await {
                            Ok(resp) => {
                                let ok = resp
                                    .get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                if ok {
                                    tui.status_msg = format!("'{}' succeeded", action_name);
                                } else {
                                    let err = resp
                                        .get("error")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown error");
                                    tui.status_msg = format!("'{}' failed: {}", action_name, err);
                                }
                            }
                            Err(e) => {
                                tui.status_msg = format!("Error: {}", e);
                            }
                        }

                        // Refresh frame after action
                        if let Ok((frame, actions)) =
                            fetch_tui_frame_direct(client, cfg, &tui).await
                        {
                            tui.last_frame = frame;
                            if !actions.is_empty() {
                                tui.actions = actions;
                            }
                        }
                    } else {
                        tui.status_msg = "Cancelled.".to_string();
                    }
                    render_tui(&tui);
                    continue;
                }

                // Normal key handling
                let mut need_refresh = false;

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up => {
                        tui.selected = (tui.selected - 1).max(0);
                        tui.status_msg.clear();
                        need_refresh = true;
                    }
                    KeyCode::Down => {
                        tui.selected += 1;
                        tui.status_msg.clear();
                        need_refresh = true;
                    }
                    KeyCode::PageUp => {
                        tui.scroll = (tui.scroll - 20).max(0);
                        tui.status_msg.clear();
                        need_refresh = true;
                    }
                    KeyCode::PageDown => {
                        tui.scroll += 20;
                        tui.status_msg.clear();
                        need_refresh = true;
                    }
                    KeyCode::Home => {
                        tui.selected = 0;
                        tui.scroll = 0;
                        tui.status_msg.clear();
                        need_refresh = true;
                    }
                    KeyCode::Char(ch) => {
                        // Look up action hotkey
                        if let Some(action) = tui.find_action_by_key(ch) {
                            let confirm = action
                                .get("confirm")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true);
                            if confirm {
                                tui.confirming = Some(action.clone());
                                render_tui(&tui);
                                continue;
                            } else {
                                // Execute without confirmation
                                let action_name = action
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                tui.status_msg =
                                    format!("Executing '{}'...", action_name);
                                render_tui(&tui);

                                match post_tui_action(client, cfg, &tui, &action_name, None).await {
                                    Ok(_) => {
                                        tui.status_msg =
                                            format!("'{}' done", action_name);
                                    }
                                    Err(e) => {
                                        tui.status_msg = format!("Error: {}", e);
                                    }
                                }
                                need_refresh = true;
                            }
                        }
                    }
                    _ => {}
                }

                if need_refresh {
                    match fetch_tui_frame_direct(client, cfg, &tui).await {
                        Ok((frame, actions)) => {
                            tui.last_frame = frame;
                            if !actions.is_empty() {
                                tui.actions = actions;
                            }
                        }
                        Err(e) => {
                            tui.status_msg = format!("Error: {}", e);
                        }
                    }
                    render_tui(&tui);
                }
            }
        }
    }

    // Cleanup
    sse_handle.abort();
    let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    println!("Back to chat.");
}

// ── Main ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    println!("StarkBot CLI");
    println!("Connected to: {}", cfg.base_url);
    println!("Type /help for commands, or just type a message.\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("stark> ");
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
            _ => {}
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            "/quit" | "/exit" | "/q" => break,
            "/help" | "/h" => cmd_help().await,
            "/modules list" | "/modules" => cmd_modules_list(&client, &cfg).await,
            "/session new" => cmd_session_new(&client, &cfg).await,
            cmd if cmd.starts_with("/modules connect ") => {
                let name = cmd.strip_prefix("/modules connect ").unwrap().trim();
                if name.is_empty() {
                    eprintln!("Usage: /modules connect <module_name>");
                } else {
                    run_tui_mode(&client, &cfg, name).await;
                }
            }
            _msg => {
                cmd_chat_stream(&client, &cfg, trimmed).await;
            }
        }
    }

    println!("Goodbye.");
}
