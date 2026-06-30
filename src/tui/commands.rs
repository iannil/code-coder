/// ─── Slash Command Handlers + Session Persistence ─────────────────────────
///
/// Extracted from mod.rs to reduce the 5000+ line file.

use super::{MessageItem, TuiApp};
use std::sync::PoisonError;

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
        // Show latest session if no id given
        let store = match app.session_store {
            Some(ref store) => store,
            None => {
                app.messages.push(MessageItem::System {
                    text: "Session store not available.".into(),
                });
                return;
            }
        };

        match store.latest() {
            Some(session) => {
                app.messages = session.messages.clone();
                app.current_session_id = Some(session.id.clone());
                app.messages.push(MessageItem::System {
                    text: format!("↻ Resumed session {} ({} msgs)", &session.id[..8], session.message_count),
                });
            }
            None => {
                app.messages.push(MessageItem::System {
                    text: "No saved sessions to resume.".into(),
                });
            }
        }
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

/// Auto-save the current session. On failure, logs to codecoder.log and pushes
/// a System message to the message list — but only on the *first* occurrence of
/// any given error string, to avoid flooding the chat during persistent disk
/// issues. Successful save resets the dedup state so a later new failure shows.
pub fn auto_save_session(app: &mut TuiApp) {
    let Some(ref store) = app.session_store else { return };
    let session = build_session_from_app(app);
    match store.save(&session) {
        Ok(()) => app.last_save_error = None,
        Err(e) => {
            let err_str = e.to_string();
            crate::log(&format!("[error] session auto-save failed: {err_str}"));
            let is_new = app.last_save_error.as_deref() != Some(err_str.as_str());
            if is_new {
                app.messages.push(MessageItem::System {
                    text: format!("[error] session auto-save failed: {err_str}"),
                });
                app.last_save_error = Some(err_str);
            }
        }
    }
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
