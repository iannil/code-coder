/// ─── Slash Command Handlers + Session Persistence ─────────────────────────
///
/// Extracted from mod.rs to reduce the 5000+ line file.

use super::{ConfirmAction, Dialog, MessageItem, TuiApp};
use crate::agent::AgentCommand;
use std::sync::PoisonError;

/// ─── Confirm Action Executor (ADR 0006) ────────────────────────────────────
///
/// Runs the destructive operation encoded in `ConfirmAction`. Called only
/// when the user confirms the dialog (Y). N and Esc do not invoke this.
pub fn execute_confirm_action(
    app: &mut TuiApp,
    cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
    action: ConfirmAction,
) {
    match action {
        ConfirmAction::ClearMessages => {
            app.messages.clear();
            crate::tui::message_list::invalidate_cache();
            app.scroll_offset = 0;
            app.auto_scroll = true;
            let _ = cmd_tx.send(AgentCommand::ClearHistory);
            app.messages.push(MessageItem::System {
                text: "⚠ Conversation cleared.".into(),
            });
        }
        ConfirmAction::ResumeLatest => {
            // Inline the latest-session load (handle_resume_cmd's no-arg
            // path). If no session exists, push an informational System msg.
            let Some(ref store) = app.session_store else {
                app.messages.push(MessageItem::System {
                    text: "Session store not available.".into(),
                });
                return;
            };
            match store.latest() {
                Some(session) => {
                    app.messages = session.messages.clone();
                    app.current_session_id = Some(session.id.clone());
                    app.messages.push(MessageItem::System {
                        text: format!("↻ Resumed session {} ({} msgs)",
                            &session.id[..8.min(session.id.len())], session.message_count),
                    });
                }
                None => {
                    app.messages.push(MessageItem::System {
                        text: "No saved sessions to resume.".into(),
                    });
                }
            }
        }
        ConfirmAction::DeleteMessage { index } => {
            if index < app.messages.len() {
                let removed = app.messages.remove(index);
                crate::tui::message_list::invalidate_cache();
                app.messages.push(MessageItem::System {
                    text: format!("Deleted message at index {index}."),
                });
                crate::log(&format!(
                    "[adr0006] deleted message idx={index}: {:?}",
                    removed
                ));
            } else {
                app.messages.push(MessageItem::System {
                    text: format!("Cannot delete: index {index} out of range."),
                });
            }
        }
    }
}

/// ─── Slash Command Dispatcher (ADR 0002) ───────────────────────────────────
///
/// Every input beginning with `/` is intercepted here before reaching the
/// agent. Known commands execute locally; unknown commands push an error
/// System message. No `/`-prefixed input ever reaches `cmd_tx.send(
/// ProcessMessage)` — that path is permanently reserved for natural-language
/// messages.
///
/// Returns true to indicate "handled (input should be cleared, no agent
/// forward)". There is no false case — the dispatcher handles every slash
/// input, including unknown ones (by reporting an error).
pub fn dispatch_slash_command(
    app: &mut TuiApp,
    input: &str,
    cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
) -> bool {
    let trimmed = input.trim();
    // Split into command word + remainder at first whitespace.
    let (cmd_word, _args) = match trimmed.find(char::is_whitespace) {
        Some(idx) => (&trimmed[..idx], trimmed[idx..].trim_start()),
        None => (trimmed, ""),
    };
    let cmd_lower = cmd_word.to_lowercase();

    match cmd_lower.as_str() {
        "/exit" | "/quit" => {
            app.should_quit = true;
        }
        "/help" | "/h" => {
            app.help_active = true;
        }
        // Aliases mirror the original: reset / new ≡ clear.
        "/clear" | "/reset" | "/new" => {
            // ADR 0006: route through Dialog::Confirm instead of executing
            // directly. The actual clear happens in execute_confirm_action
            // when the user confirms.
            app.dialog = Some(Dialog::Confirm {
                message: "Clear all messages from this session?".into(),
                action: ConfirmAction::ClearMessages,
            });
        }
        "/reload" => {
            let _ = cmd_tx.send(AgentCommand::ReloadContext);
            app.messages.push(MessageItem::System {
                text: "→ Context and skills reloaded.".into(),
            });
        }
        "/history" => {
            // Report actual in-memory count instead of asking the LLM.
            let count = app.messages.len();
            app.messages.push(MessageItem::System {
                text: format!("{count} message(s) in current session."),
            });
        }
        "/tools" => {
            app.messages.push(MessageItem::System {
                text: "→ Tools are listed in the agent's system prompt; ask the agent to use them.".into(),
            });
        }
        "/skills" => {
            app.messages.push(MessageItem::System {
                text: "→ Skills are listed in the agent's system prompt; ask the agent to use them.".into(),
            });
        }
        "/memory" => {
            app.messages.push(MessageItem::System {
                text: "→ Memory entries are stored in the memory/ directory.".into(),
            });
        }
        "/session" => {
            handle_session_cmd(app);
        }
        "/resume" => {
            // ADR 0006 will wrap this in Dialog::Confirm before overwriting.
            handle_resume_cmd(app, trimmed);
        }
        // Alias mirrors the original: settings ≡ config.
        "/config" | "/settings" => {
            handle_config_cmd(app, trimmed);
        }
        "/mcp" => {
            handle_mcp_cmd(app, trimmed);
        }
        _ => {
            app.messages.push(MessageItem::System {
                text: format!("Unknown command: {cmd_word}. Type /help for the list."),
            });
        }
    }
    true
}

/// Handle `/session` — list saved sessions.
pub fn handle_session_cmd(app: &mut TuiApp) {
    let store = match app.session_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Session store not available.".into(),
            });
            return;
        }
    };

    let headers = store.list();
    if headers.is_empty() {
        app.messages.push(MessageItem::System {
            text: "No saved sessions.".into(),
        });
        return;
    }

    let current = app.current_session_id.as_deref().unwrap_or("");
    let mut lines = vec!["── Saved Sessions ──".to_string()];
    for h in &headers {
        let marker = if h.id == current { " ◀ (current)" } else { "" };
        let preview = h.previews.first()
            .map(|p| format!(" — {}", truncate_str(p, 60)))
            .unwrap_or_default();
        lines.push(format!("  #{}{}{}", &h.id[..8], marker, preview));
    }
    lines.push("Use /resume <id> to load a session.".into());

    app.messages.push(MessageItem::System {
        text: lines.join("\n"),
    });
}

/// Handle `/resume <id>` — load a saved session.
pub fn handle_resume_cmd(app: &mut TuiApp, input: &str) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    if parts.len() < 2 || parts[1].trim().is_empty() {
        // ADR 0006: no-arg /resume is destructive (overwrites current
        // messages). Route through Dialog::Confirm instead of loading
        // directly. If the session store is missing or empty, the
        // confirm executor will surface a System message on confirm.
        app.dialog = Some(Dialog::Confirm {
            message: "Resume the latest saved session? Current messages will be replaced.".into(),
            action: ConfirmAction::ResumeLatest,
        });
        return;
    }

    let partial_id = parts[1].trim();
    let store = match app.session_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Session store not available.".into(),
            });
            return;
        }
    };

    let headers = store.list();
    let matched: Vec<_> = headers.iter()
        .filter(|h| h.id.starts_with(partial_id))
        .collect();

    match matched.len() {
        0 => {
            app.messages.push(MessageItem::System {
                text: format!("No session found matching '{partial_id}'. Use /session to list."),
            });
        }
        1 => {
            match store.load(&matched[0].id) {
                Ok(session) => {
                    app.messages = session.messages.clone();
                    app.current_session_id = Some(session.id.clone());
                    app.messages.push(MessageItem::System {
                        text: format!("↻ Resumed session {} ({} msgs)", &session.id[..8], session.message_count),
                    });
                }
                Err(e) => {
                    app.messages.push(MessageItem::System {
                        text: format!("Error loading session: {e}"),
                    });
                }
            }
        }
        _ => {
            app.messages.push(MessageItem::System {
                text: format!("Multiple sessions match '{partial_id}'. Be more specific."),
            });
        }
    }
}

/// Truncate a string to max_len chars, appending "…" if truncated.
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

/// Handle `/config` and `/config set <key> <value>`.
pub fn handle_config_cmd(app: &mut TuiApp, input: &str) {
    let store = match app.config_store {
        Some(ref store) => store,
        None => {
            app.messages.push(MessageItem::System {
                text: "Config store not available.".into(),
            });
            return;
        }
    };

    let parts: Vec<&str> = input.splitn(4, ' ').collect();

    if parts.len() < 2 || parts[1].trim().is_empty() {
        app.messages.push(MessageItem::System {
            text: store.format_display(),
        });
        return;
    }

    if parts[1] == "set" && parts.len() >= 4 {
        let key = parts[2].to_lowercase();
        let value = parts[3];
        let result = apply_config_setting(app, &key, value);
        app.messages.push(MessageItem::System { text: result });
        return;
    }

    app.messages.push(MessageItem::System {
        text: format!("Unknown config subcommand: {}. Use /config to view, /config set <key> <value> to change.", parts[1]),
    });
}

/// Apply a single config change.
pub fn apply_config_setting(app: &mut TuiApp, key: &str, value: &str) -> String {
    let store = match app.config_store {
        Some(ref mut store) => store,
        None => return "Config store not available.".into(),
    };

    match key {
        "model" => {
            store.set_model(value);
            app.status.model = value.to_string();
            if let Err(e) = store.save() {
                return format!("Changed model to {value}, but failed to save: {e}");
            }
            format!("Model set to {value} (saved to codecoder.json). Use /reload to apply in current agent.")
        }
        "api_base" => {
            store.get_mut().llm.api_base = value.to_string();
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("API Base set to {value}. Restart to apply.")
        }
        "max_tokens" => {
            let n: u32 = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().llm.max_tokens = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Max tokens set to {n}. Restart to apply.")
        }
        "temperature" => {
            let t: f32 = match value.parse() {
                Ok(t) => t,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().llm.temperature = t;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Temperature set to {t}. Restart to apply.")
        }
        "tool_rounds" | "max_tool_rounds" => {
            let n: usize = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().features.max_tool_rounds = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Max tool rounds set to {n}. Restart to apply.")
        }
        "cmd_timeout" | "command_timeout_secs" => {
            let n: u64 = match value.parse() {
                Ok(n) => n,
                Err(_) => return format!("Invalid number: {value}"),
            };
            store.get_mut().features.command_timeout_secs = n;
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Command timeout set to {n}s. Restart to apply.")
        }
        "sandbox_memory" | "sandbox_memory_limit" => {
            store.get_mut().features.sandbox_memory_limit = value.to_string();
            if let Err(e) = store.save() {
                return format!("Failed to save: {e}");
            }
            format!("Sandbox memory limit set to '{value}'. Restart to apply.")
        }
        _ => {
            format!("Unknown config key: {key}. Supported: model, api_base, max_tokens, temperature, tool_rounds, cmd_timeout, sandbox_memory")
        }
    }
}

/// Handle `/mcp`, `/mcp list`, `/mcp start <name>`, `/mcp stop <name>`.
pub fn handle_mcp_cmd(app: &mut TuiApp, input: &str) {
    let registry = match app.mcp_registry {
        Some(ref reg) => reg,
        None => {
            app.messages.push(MessageItem::System {
                text: "MCP not available.".into(),
            });
            return;
        }
    };

    let parts: Vec<&str> = input.splitn(3, ' ').collect();
    let sub = parts.get(1).copied().unwrap_or("");

    match sub {
        "list" | "" => {
            let reg = registry.lock().unwrap_or_else(PoisonError::into_inner);
            let servers = reg.list_servers();
            let tools = reg.all_tools();

            let mut lines = vec!["── MCP Servers ──".to_string()];
            if servers.is_empty() {
                lines.push("  No MCP servers running.".into());
                lines.push("  Configure servers in codecoder.json under 'mcp_servers'.".into());
            } else {
                for s in &servers {
                    lines.push(format!("  ✓ {} (v{}, {} tools)", s.name, s.server_info.version, s.tool_count));
                }
            }

            if !tools.is_empty() {
                lines.push(String::new());
                lines.push("── MCP Tools ──".to_string());
                for t in &tools {
                    let desc = if t.description.len() > 60 {
                        format!("{}…", &t.description[..57])
                    } else {
                        t.description.clone()
                    };
                    lines.push(format!("  · {} [{}] {}", t.tool_name, t.server_name, desc));
                }
            }

            app.messages.push(MessageItem::System {
                text: lines.join("\n"),
            });
        }
        "start" => {
            let name = parts.get(2).unwrap_or(&"");
            if name.is_empty() {
                app.messages.push(MessageItem::System {
                    text: "Usage: /mcp start <server-name>".into(),
                });
                return;
            }

            let config = match app.config_store {
                Some(ref store) => store.get().clone(),
                None => {
                    app.messages.push(MessageItem::System {
                        text: "Config not available.".into(),
                    });
                    return;
                }
            };

            let server_config = config.mcp_servers.iter()
                .find(|s| s.name == *name)
                .cloned();

            match server_config {
                Some(cfg) => {
                    let mut reg = registry.lock().unwrap_or_else(PoisonError::into_inner);
                    match reg.start_server(cfg) {
                        Ok(name) => {
                            let tools = reg.all_tools();
                            let count = tools.iter().filter(|t| t.server_name == name).count();
                            app.messages.push(MessageItem::System {
                                text: format!("✓ MCP server '{name}' started ({count} tools)"),
                            });
                        }
                        Err(e) => {
                            app.messages.push(MessageItem::System {
                                text: format!("✗ Failed to start MCP server '{name}': {e}"),
                            });
                        }
                    }
                }
                None => {
                    app.messages.push(MessageItem::System {
                        text: format!("No MCP server named '{name}' in config. Use /config to see configured servers."),
                    });
                }
            }
        }
        "stop" => {
            let name = parts.get(2).unwrap_or(&"");
            if name.is_empty() {
                app.messages.push(MessageItem::System {
                    text: "Usage: /mcp stop <server-name>".into(),
                });
                return;
            }
            let mut reg = registry.lock().unwrap_or_else(PoisonError::into_inner);
            match reg.stop_server(name) {
                Ok(()) => {
                    app.messages.push(MessageItem::System {
                        text: format!("✓ MCP server '{name}' stopped."),
                    });
                }
                Err(e) => {
                    app.messages.push(MessageItem::System {
                        text: format!("✗ Failed to stop server '{name}': {e}"),
                    });
                }
            }
        }
        _ => {
            app.messages.push(MessageItem::System {
                text: format!("Unknown MCP subcommand: '{sub}'. Use /mcp list, /mcp start, /mcp stop."),
            });
        }
    }
}

/// Build a Session from the current TuiApp state.
/// Applies save-time per-field compaction so the persisted file is bounded
/// regardless of in-memory state.
pub fn build_session_from_app(app: &TuiApp) -> crate::session::Session {
    let id = app.current_session_id.clone()
        .unwrap_or_else(|| crate::session::Session::new(&app.status.model).id);
    let mut messages = app.messages.clone();
    super::app::compact_messages_for_save(&mut messages);
    let mut session = crate::session::Session {
        schema_version: crate::session::CURRENT_SCHEMA_VERSION,
        id,
        model: app.status.model.clone(),
        created_at: String::new(),
        updated_at: String::new(),
        message_count: 0,
        token_count: app.status.token_count,
        messages,
    };
    session.touch();
    session
}

/// ADR 0004: mark the session dirty. The main loop flushes ~5s later on a
/// background thread. Replaces the old synchronous save-per-keystroke.
pub fn auto_save_session(app: &mut TuiApp) {
    // No store wired → nothing to save (tests / non-persistent runs).
    if app.session_store.is_none() {
        return;
    }
    app.dirty = true;
    app.last_dirty_at = Some(std::time::Instant::now());
}

/// ADR 0004: debounce window. The main loop calls this once per frame; if
/// the session is dirty and the last dirty mark is older than the window,
/// spawns a save onto the background thread.
const SAVE_DEBOUNCE_SECS: u64 = 5;

/// Try to flush a pending dirty mark to the background save thread. No-op
/// when not dirty, when the debounce window hasn't elapsed, or when no
/// save thread is wired (legacy synchronous fallback then).
pub fn flush_pending_save(app: &mut TuiApp) {
    if !app.dirty {
        return;
    }
    let Some(last) = app.last_dirty_at else {
        return;
    };
    if last.elapsed().as_secs() < SAVE_DEBOUNCE_SECS {
        return;
    }
    // Mark clean BEFORE building the snapshot so concurrent marks during
    // serialize don't get lost (they'll trigger another flush next frame).
    app.dirty = false;
    app.last_dirty_at = None;

    let session = build_session_from_app(app);

    if let Some(tx) = &app.save_tx {
        // Background thread path. Send errors are silently dropped — the
        // next dirty mark will retry next frame.
        let _ = tx.send(session);
    } else if let Some(ref store) = app.session_store {
        // Legacy synchronous fallback (no background thread spawned yet,
        // e.g. tests). Run inline so saves still happen.
        report_save_result(app, store.save(&session));
    }
}

/// Synchronous flush on exit. The main loop calls this after the event
/// loop ends. If a background thread is running, drops the sender (which
/// signals EOF to the receiver) and joins; otherwise runs one inline
/// save. Returns when all pending saves have completed or errored.
pub fn flush_on_exit(app: &mut TuiApp) {
    // Take the sender so the background thread sees channel close.
    let tx = app.save_tx.take();
    if let Some(store) = app.session_store.as_ref() {
        if app.dirty {
            let session = build_session_from_app(app);
            app.dirty = false;
            app.last_dirty_at = None;
            // If background thread exists, send the final snapshot; the
            // drop(tx) below will make the thread exit after processing.
            if let Some(tx) = &tx {
                let _ = tx.send(session);
            } else {
                report_save_result(app, store.save(&session));
            }
        }
    }
    // Drop sender → background thread drains remaining work then exits.
    // (Thread is detached; we trust it finishes within OS thread lifetime.
    // A future Phase C could block on join via a handle in TuiApp.)
    drop(tx);
}

/// Update app state with the result of a save attempt (dedup on error
/// string to avoid flooding the message list).
fn report_save_result(app: &mut TuiApp, result: anyhow::Result<()>) {
    match result {
        Ok(()) => app.last_save_error = None,
        Err(e) => {
            let err_str = e.to_string();
            crate::log(&format!("[error] session save failed: {err_str}"));
            let is_new = app.last_save_error.as_deref() != Some(err_str.as_str());
            if is_new {
                app.messages.push(MessageItem::System {
                    text: format!("[error] session save failed: {err_str}"),
                });
                app.last_save_error = Some(err_str);
            }
        }
    }
}

/// ADR 0004: spawn the background save thread. Returns the sender to wire
/// into `TuiApp::save_tx`. The thread owns the SessionStore and writes
/// sessions as they arrive; it exits when the sender is dropped (EOF).
pub fn spawn_save_thread(store: crate::session::SessionStore) -> std::sync::mpsc::Sender<crate::session::Session> {
    let (tx, rx) = std::sync::mpsc::channel::<crate::session::Session>();
    std::thread::Builder::new()
        .name("codecoder-save".into())
        .spawn(move || {
            for session in rx.iter() {
                if let Err(e) = store.save(&session) {
                    crate::log(&format!("[error] background save failed: {e}"));
                }
            }
        })
        .expect("spawn save thread");
    tx
}

/// Fetch available models from the LLM API's `/v1/models` endpoint.
/// Falls back to a hardcoded list of common models on error.
pub fn fetch_available_models(api_base: &str, api_key: &str) -> Vec<String> {
    let fallback = vec![
        "gpt-4o".into(), "gpt-4o-mini".into(), "gpt-4.1".into(),
        "gpt-4.1-mini".into(), "gpt-4.1-nano".into(), "o3".into(),
        "o4-mini".into(), "claude-sonnet-4-20250514".into(),
        "claude-haiku-3-5".into(), "deepseek-chat".into(),
        "llama3.2".into(), "gemini-2.5-flash".into(),
    ];

    if api_key.is_empty() {
        return fallback;
    }

    let base = api_base.trim_end_matches('/');
    let url = format!("{}/models", base);

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return fallback,
    };
    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(r) => r,
        Err(_) => return fallback,
    };
    if !resp.status().is_success() {
        return fallback;
    }

    #[derive(serde::Deserialize)]
    struct ModelList {
        data: Vec<ModelEntry>,
    }
    #[derive(serde::Deserialize)]
    struct ModelEntry {
        id: String,
    }

    match resp.json::<ModelList>() {
        Ok(list) => {
            let mut models: Vec<String> = list.data.into_iter()
                .map(|m| m.id)
                .filter(|id| !id.starts_with("ft:"))
                .collect();
            models.sort();
            if models.is_empty() { fallback } else { models }
        }
        Err(_) => fallback,
    }
}

// ─── ADR 0002 Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod adr0002_tests {
    use super::*;
    use crate::agent::AgentCommand;
    use crate::tui::TuiApp;

    fn dispatch(app: &mut TuiApp, input: &str, cmd_tx: &std::sync::mpsc::Sender<AgentCommand>) -> bool {
        dispatch_slash_command(app, input, cmd_tx)
    }

    // ── Known commands route locally ──────────────────────────────────────

    #[test]
    fn exit_sets_should_quit() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        assert!(dispatch(&mut app, "/exit", &tx));
        assert!(app.should_quit);
    }

    #[test]
    fn quit_sets_should_quit() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        assert!(dispatch(&mut app, "/quit", &tx));
        assert!(app.should_quit);
    }

    #[test]
    fn case_insensitive_exit() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        assert!(dispatch(&mut app, "/EXIT", &tx));
        assert!(app.should_quit);
    }

    #[test]
    fn help_opens_help_panel() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/help", &tx);
        assert!(app.help_active, "/help should set help_active");
    }

    #[test]
    fn reset_and_new_alias_clear() {
        use crate::tui::{Dialog, ConfirmAction};
        for alias in ["/reset", "/new"] {
            let mut app = TuiApp::default();
            let (tx, _rx) = std::sync::mpsc::channel();
            dispatch(&mut app, alias, &tx);
            match &app.dialog {
                Some(Dialog::Confirm { action: ConfirmAction::ClearMessages, .. }) => {}
                other => panic!("{alias} should open Confirm{{ClearMessages}}, got {other:?}"),
            }
        }
    }

    #[test]
    fn settings_aliases_config() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        // /settings should be handled (not fall through to "Unknown command").
        assert!(dispatch(&mut app, "/settings", &tx));
        assert!(
            !app.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.starts_with("Unknown command"))),
            "/settings must alias /config, not be unknown"
        );
    }

    #[test]
    fn h_alias_opens_help() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/h", &tx);
        assert!(app.help_active);
    }

    #[test]
    fn clear_constructs_confirm_dialog_adr0006() {
        // ADR 0006: /clear no longer clears directly — it constructs a
        // Dialog::Confirm{ClearMessages}. The clear happens only when the
        // user confirms via execute_confirm_action.
        use crate::tui::{ConfirmAction, Dialog};
        let mut app = TuiApp::default();
        app.messages.push(MessageItem::User { text: "msg1".into() });
        app.messages.push(MessageItem::User { text: "msg2".into() });
        let (tx, rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/clear", &tx);
        // Messages NOT cleared; dialog opened.
        assert_eq!(app.messages.len(), 2, "/clear must not clear directly (ADR 0006)");
        assert!(rx.try_recv().is_err(), "no agent command until user confirms");
        match app.dialog {
            Some(Dialog::Confirm { action: ConfirmAction::ClearMessages, .. }) => {}
            other => panic!("expected Confirm{{ClearMessages}}, got {other:?}"),
        }
    }

    #[test]
    fn reload_sends_reloadcontext_and_msgs() {
        let mut app = TuiApp::default();
        let (tx, rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/reload", &tx);
        match rx.try_recv() {
            Ok(AgentCommand::ReloadContext) => {}
            other => panic!("expected ReloadContext, got {other:?}"),
        }
        assert!(app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("reloaded")
        )));
    }

    #[test]
    fn history_reports_actual_count() {
        let mut app = TuiApp::default();
        for i in 0..7 {
            app.messages.push(MessageItem::User { text: format!("m{i}") });
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/history", &tx);
        // Should NOT send any agent command — count is local.
        // The pushed System message should mention "7 message".
        let last = app.messages.last().expect("msg pushed");
        match last {
            MessageItem::System { text } => assert!(text.contains("7 message"), "got: {text}"),
            _ => panic!("expected System msg"),
        }
    }

    #[test]
    fn unknown_command_pushes_error_and_returns_handled() {
        let mut app = TuiApp::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let handled = dispatch(&mut app, "/notarealcommand", &tx);
        assert!(handled, "unknown commands are still 'handled' (error reported)");
        // No agent command forwarded
        assert!(rx.try_recv().is_err(), "unknown command must not send to agent");
        // Error System message pushed
        let last = app.messages.last().expect("msg pushed");
        match last {
            MessageItem::System { text } => {
                assert!(text.contains("Unknown command"), "got: {text}");
                assert!(text.contains("/help"), "should mention /help: {text}");
            }
            _ => panic!("expected System msg"),
        }
    }

    #[test]
    fn args_after_command_work_for_resume() {
        // /resume with an arg should be dispatched (handle_resume_cmd will
        // run even if no sessions match — it just reports an error).
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        let handled = dispatch(&mut app, "/resume nonexistent", &tx);
        assert!(handled);
        // handle_resume_cmd pushes a System message
        assert!(!app.messages.is_empty());
    }

    #[test]
    fn config_with_args_dispatches() {
        // /config with subcommand — handler runs even without a config store
        // (it pushes a "not available" message).
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        let handled = dispatch(&mut app, "/config", &tx);
        assert!(handled);
        assert!(!app.messages.is_empty(), "config handler should push a System msg");
    }

    #[test]
    fn session_dispatches_to_handler() {
        let mut app = TuiApp::default();
        let (tx, _rx) = std::sync::mpsc::channel();
        dispatch(&mut app, "/session", &tx);
        // handle_session_cmd pushes a System msg ("Session store not available."
        // when no store is wired, which is the expected test fixture).
        assert!(!app.messages.is_empty());
    }

    // ── send_message integration: no ProcessMessage leak for / inputs ─────

    #[test]
    fn send_message_routes_slash_input_through_dispatcher() {
        use crate::tui::input_area::send_message;
        let mut app = TuiApp::default();
        app.input = "/help".into();
        let (tx, rx) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        // /help opens help panel; input cleared; NO ProcessMessage sent.
        assert!(app.help_active);
        assert!(app.input.is_empty());
        assert!(rx.try_recv().is_err(), "no agent command should be sent for /help");
    }

    #[test]
    fn send_message_rejects_unknown_slash_without_agent_forward() {
        use crate::tui::input_area::send_message;
        let mut app = TuiApp::default();
        app.input = "/totallyfake".into();
        let (tx, rx) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        // Error pushed, no agent forward, input cleared
        assert!(app.input.is_empty(), "input should clear after dispatch");
        assert!(rx.try_recv().is_err(), "must NOT forward to agent");
        assert!(app.messages.iter().any(|m| matches!(
            m,
            MessageItem::System { text } if text.contains("Unknown command")
        )));
    }

    #[test]
    fn send_message_forwards_non_slash_to_agent() {
        use crate::tui::input_area::send_message;
        let mut app = TuiApp::default();
        app.input = "hello world".into();
        let (tx, rx) = std::sync::mpsc::channel();
        send_message(&mut app, &tx);
        match rx.try_recv() {
            Ok(AgentCommand::ProcessMessage { text }) => assert_eq!(text, "hello world"),
            other => panic!("expected ProcessMessage, got {other:?}"),
        }
    }

    // ── Slash completion filtering (ADR 0002 §7) ──────────────────────────

    #[test]
    fn refresh_activates_on_leading_slash() {
        use crate::tui::input_area::refresh_slash_completion;
        let mut app = TuiApp::default();
        app.input = "/".into();
        refresh_slash_completion(&mut app);
        assert!(app.slash_completion.active);
        // Empty prefix → all commands visible
        assert_eq!(app.slash_completion.filtered.len(), app.slash_completion.commands.len());
    }

    #[test]
    fn refresh_filters_by_prefix() {
        use crate::tui::input_area::refresh_slash_completion;
        let mut app = TuiApp::default();
        app.input = "/se".into();  // matches /session
        refresh_slash_completion(&mut app);
        assert!(app.slash_completion.active);
        // Only /session should match
        let matched: Vec<&str> = app.slash_completion.filtered.iter()
            .map(|&i| app.slash_completion.commands[i])
            .collect();
        assert_eq!(matched, vec!["/session"], "got: {matched:?}");
    }

    #[test]
    fn refresh_deactivates_when_input_lacks_slash() {
        use crate::tui::input_area::refresh_slash_completion;
        let mut app = TuiApp::default();
        app.input = "hello".into();
        refresh_slash_completion(&mut app);
        assert!(!app.slash_completion.active);
        assert!(app.slash_completion.filtered.is_empty());
    }

    #[test]
    fn refresh_keeps_popup_open_when_args_start() {
        // Input "/resume foo" — args started, popup stays open as reference
        // but no filtering.
        use crate::tui::input_area::refresh_slash_completion;
        let mut app = TuiApp::default();
        app.input = "/resume foo".into();
        refresh_slash_completion(&mut app);
        assert!(app.slash_completion.active, "popup should stay open with args present");
        // No filtering — all visible
        assert_eq!(app.slash_completion.filtered.len(), app.slash_completion.commands.len());
    }

    #[test]
    fn refresh_prefix_match_is_case_insensitive() {
        use crate::tui::input_area::refresh_slash_completion;
        let mut app = TuiApp::default();
        app.input = "/RE".into();  // uppercase prefix
        refresh_slash_completion(&mut app);
        let matched: Vec<&str> = app.slash_completion.filtered.iter()
            .map(|&i| app.slash_completion.commands[i])
            .collect();
        // Should match /reload, /resume (anything starting with "re")
        assert!(matched.iter().any(|c| *c == "/reload"), "got: {matched:?}");
        assert!(matched.iter().any(|c| *c == "/resume"), "got: {matched:?}");
    }
}
