//! rr-mcp — Rational Reserve MCP stdio shim.
//!
//! Host agents (Claude Code, Codex, Qwen-Code, etc.) spawn this as a subprocess.
//! It:
//!   1. Calls `ensure_daemon` to guarantee a single background daemon.
//!   2. Connects to the unix socket.
//!   3. Registers MCP tools (`rr_roster`, `rr_deploy_swarm`, `rr_status`)
//!      whose handlers proxy JSON-RPC to the daemon.

use rr::daemon::ensure_daemon;
use serde_json::json;
use std::io::{self, BufRead, Write};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Ensure the daemon is running
    let paths = ensure_daemon(None, 5.0).await?;

    // Connect to the daemon
    let sock_path = paths.socket_path();
    let _stream = UnixStream::connect(&sock_path).await?;

    // MCP protocol: read JSON-RPC from stdin, forward to daemon over unix socket,
    // return results to stdout.
    //
    // For v0.1 we implement a minimal MCP-compatible stdio server that registers
    // three tools. A full MCP SDK integration would be ideal but is deferred.

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("rr-mcp shim connected to daemon at {}", sock_path.display());

    // Simple read loop: each line from stdin is a JSON-RPC request.
    // For v0.1, we proxy directly to the daemon and forward responses.
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: serde_json::Value = serde_json::from_str(&line)?;
        let method = request.get("method").and_then(|v| v.as_str());
        let params = request.get("params").cloned().unwrap_or(json!({}));
        let req_id = request.get("id").cloned();

        let Some(method) = method else {
            let resp = json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {"code": -32600, "message": "no method specified"},
            });
            writeln!(stdout, "{}", resp)?;
            stdout.flush()?;
            continue;
        };

        // Map MCP tool names to daemon methods
        let (daemon_method, daemon_params) = match method {
            "tools/call" => {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
                match name {
                    "rr_roster" => ("rr_roster", arguments),
                    "rr_deploy_swarm" => ("rr_deploy_swarm", arguments),
                    "rr_status" => ("rr_status", arguments),
                    "rr_execute_mission" => ("rr_execute_mission", arguments),
                    "rr_disband" => ("rr_disband", arguments),
                    "cr_register_civilian" => ("cr_register_civilian", arguments),
                    "cr_list_civilians" => ("cr_list_civilians", arguments),
                    "cr_firm_detail" => ("cr_firm_detail", arguments),
                    "cr_contract_request" => ("cr_contract_request", arguments),
                    "cr_contract_accept" => ("cr_contract_accept", arguments),
                    "cr_contract_sitrep" => ("cr_contract_sitrep", arguments),
                    "cr_contract_closeout" => ("cr_contract_closeout", arguments),
                    "cr_execute_contract" => ("cr_execute_contract", arguments),
                    "initialize" => {
                        // Respond to initialization
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "protocolVersion": "2024-11-05",
                                "capabilities": {"tools": {}},
                                "serverInfo": {"name": "rational-reserve", "version": "0.1.0"},
                            },
                        });
                        writeln!(stdout, "{}", resp)?;
                        stdout.flush()?;
                        continue;
                    }
                    "tools/list" => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "tools": [
                                    {
                                        "name": "rr_roster",
                                        "description": "List agents in a swarm with optional filters",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "swarm_id": {"type": "string"},
                                                "status": {"type": "string"},
                                                "rank": {"type": "string"},
                                                "mos": {"type": "string"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "rr_deploy_swarm",
                                        "description": "Deploy a new swarm with the given formation",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["formation", "primary_mos"],
                                            "properties": {
                                                "formation": {"type": "string", "enum": ["single", "fire_team", "squad", "platoon"]},
                                                "primary_mos": {"type": "string"},
                                                "swarm_name": {"type": "string"},
                                                "support_mos": {"type": "string"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "rr_status",
                                        "description": "Get the live status projection for a swarm",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["swarm_id"],
                                            "properties": {
                                                "swarm_id": {"type": "string"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "rr_execute_mission",
                                        "description": "Execute a mission: plan (via LLM) -> assign -> execute DAG. Military path.",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["swarm_id", "objective"],
                                            "properties": {
                                                "swarm_id": {"type": "string"},
                                                "objective": {"type": "string"},
                                                "priority": {"type": "string", "enum": ["routine","priority","urgent","critical"]},
                                            },
                                        },
                                    },
                                    {
                                        "name": "rr_disband",
                                        "description": "Disband a swarm and generate an After-Action Review",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["swarm_id"],
                                            "properties": {"swarm_id": {"type": "string"}},
                                        },
                                    },
                                    {
                                        "name": "cr_register_civilian",
                                        "description": "Register a Civilian Reserve contractor (firm or independent) from its doctrine file",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["slug"],
                                            "properties": {
                                                "slug": {"type": "string", "description": "Civilian slug (lowercase, [a-z0-9_-])"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_list_civilians",
                                        "description": "List all registered Civilian Reserve contractors",
                                        "inputSchema": {"type": "object", "properties": {}},
                                    },
                                    {
                                        "name": "cr_firm_detail",
                                        "description": "Inspect a firm: doctrine + specialists + routing rules + open contracts",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["firm_id"],
                                            "properties": {
                                                "firm_id": {"type": "string"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_contract_request",
                                        "description": "Initiate a contract from a military agent to a civilian firm (or independent)",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["contracting_agent", "firm_id", "brief"],
                                            "properties": {
                                                "contracting_agent": {"type": "string"},
                                                "firm_id": {"type": "string"},
                                                "brief": {"type": "string"},
                                                "mission_id": {"type": "string"},
                                                "priority": {"type": "string", "enum": ["routine", "priority", "urgent", "critical"]},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_contract_accept",
                                        "description": "Firm-side: accept a Requested contract and assign to a specialist",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["contract_id", "specialist_id"],
                                            "properties": {
                                                "contract_id": {"type": "string"},
                                                "specialist_id": {"type": "string"},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_contract_sitrep",
                                        "description": "Submit a SITREP on an open contract (implicit Accepted->InProgress transition on first call)",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["contract_id"],
                                            "properties": {
                                                "contract_id": {"type": "string"},
                                                "status": {"type": "string"},
                                                "progress": {"type": "number"},
                                                "message": {"type": "string"},
                                                "blockers": {"type": "array", "items": {"type": "string"}},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_contract_closeout",
                                        "description": "Close a contract with outcome and deliverables",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["contract_id", "outcome"],
                                            "properties": {
                                                "contract_id": {"type": "string"},
                                                "outcome": {"type": "string", "enum": ["complete", "failed", "cancelled"]},
                                                "deliverables": {},
                                            },
                                        },
                                    },
                                    {
                                        "name": "cr_execute_contract",
                                        "description": "Execute a contract: build prompt from civilian doctrine + brief, call LLM, record SITREP, auto-close on completion",
                                        "inputSchema": {
                                            "type": "object",
                                            "required": ["contract_id"],
                                            "properties": {
                                                "contract_id": {"type": "string"},
                                                "auto_close": {"type": "boolean"},
                                            },
                                        },
                                    },
                                ],
                            },
                        });
                        writeln!(stdout, "{}", resp)?;
                        stdout.flush()?;
                        continue;
                    }
                    _ => {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "error": {"code": -32601, "message": format!("unknown tool: {}", name)},
                        });
                        writeln!(stdout, "{}", resp)?;
                        stdout.flush()?;
                        continue;
                    }
                }
            }
            "initialize" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "rational-reserve", "version": "0.1.0"},
                    },
                });
                writeln!(stdout, "{}", resp)?;
                stdout.flush()?;
                continue;
            }
            "ping" => ("ping", json!({})),
            _ => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": format!("unknown method: {}", method)},
                });
                writeln!(stdout, "{}", resp)?;
                stdout.flush()?;
                continue;
            }
        };

        // Forward to daemon
        let daemon_request = json!({
            "id": req_id,
            "method": daemon_method,
            "params": daemon_params,
        });

        match call_daemon(&sock_path, &daemon_request).await {
            Ok(result) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result)?,
                        }],
                    },
                });
                writeln!(stdout, "{}", resp)?;
            }
            Err(e) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32603, "message": e.to_string()},
                });
                writeln!(stdout, "{}", resp)?;
            }
        }
        stdout.flush()?;
    }

    Ok(())
}

async fn call_daemon(
    sock_path: &std::path::Path,
    request: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let stream = UnixStream::connect(sock_path).await?;
    let (read_half, mut write_half) = tokio::io::split(stream);

    let line = serde_json::to_string(request)? + "\n";
    write_half.write_all(line.as_bytes()).await?;
    write_half.flush().await?;

    let mut response_line = String::new();
    tokio::io::AsyncBufReadExt::read_line(&mut tokio::io::BufReader::new(read_half), &mut response_line)
        .await?;

    let response: serde_json::Value = serde_json::from_str(&response_line)?;

    if let Some(error) = response.get("error") {
        return Err(format!(
            "daemon error: {}",
            error.get("message").map(|v| v.to_string()).unwrap_or_default()
        )
        .into());
    }

    response
        .get("result")
        .cloned()
        .ok_or_else(|| "daemon returned malformed response".into())
}
