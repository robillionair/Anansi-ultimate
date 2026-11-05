use anansi_api::{ApiState, pairing::Session, router};
use anansi_core::Core;
use std::{path::PathBuf, sync::Arc};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = 1451_u16;
    let mut mcp_server = false;
    let mut mcp_workspace_id: Option<String> = None;
    let mut root = if cfg!(windows) {
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA missing")?)
            .join("ANANSI/pairing")
    } else {
        PathBuf::from(std::env::var_os("HOME").ok_or("HOME missing")?)
            .join(".local/share/anansi/pairing")
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => port = args.next().ok_or("Missing port")?.parse()?,
            "--pairing-root" => root = PathBuf::from(args.next().ok_or("Missing pairing root")?),
            "--mcp-server" => mcp_server = true,
            "--workspace-id" => {
                mcp_workspace_id = Some(args.next().ok_or("Missing workspace id")?)
            }
            #[cfg(debug_assertions)]
            "--seal-test-workspace" => {
                let workspace = args.next().ok_or("Missing workspaceId")?;
                let job_id = args.next().ok_or("Missing jobId")?;
                let artifact_id = args.next().ok_or("Missing artifactId")?;
                let sha256 = args.next().ok_or("Missing sha256")?;
                let core = Core::for_host();
                let storage_root = core.storage_root().map_err(|e| e.message)?;
                let vault =
                    anansi_core::vault::service::AdapterVaultState::for_storage_root(storage_root)
                        .map_err(|e| e.message)?;
                vault
                    .execute(
                        anansi_core::vault::session::Transport::Desktop,
                        "desktop-main",
                        None,
                        "setup",
                        serde_json::json!({"contractVersion":1,"workspaceId":workspace}),
                    )
                    .map_err(|e| e.message)?;
                vault
                    .execute(
                        anansi_core::vault::session::Transport::Desktop,
                        "desktop-main",
                        None,
                        "seal",
                        serde_json::json!({
                            "contractVersion":1,
                            "workspaceId":workspace,
                            "jobId":job_id,
                            "artifactId":artifact_id,
                            "sha256":sha256
                        }),
                    )
                    .map_err(|e| e.message)?;
                return Ok(());
            }
            _ => return Err("Unsupported startup argument".into()),
        }
    }
    if mcp_server {
        // MCP server mode: read-only NDJSON stdio server, disabled by
        // default. Authentication is by live pairing token: the client
        // must present the token of a currently running API pairing
        // session (pairing files vanish when their session ends, so a
        // token stops working when its session does). Stdout carries
        // protocol traffic only; diagnostics go to stderr.
        let workspace_id = mcp_workspace_id.ok_or("Missing workspace id")?;
        // The shared --pairing-root flag (or the host default when it is
        // absent) is the pairing root the server validates against. An
        // earlier revision of this branch added a second, unreachable
        // --pairing-root arm that shadowed the flag; the server then
        // silently fell back to the host default and validated against
        // unrelated sessions. The flag is now the single source.
        let pairing_root = root;
        let storage_root = std::env::var_os("ANANSI_STORAGE_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                if cfg!(windows) {
                    std::env::var_os("LOCALAPPDATA")
                        .map(|p| PathBuf::from(p).join("ANANSI/workspaces"))
                } else {
                    std::env::var_os("HOME")
                        .map(|p| PathBuf::from(p).join(".local/share/anansi/workspaces"))
                }
            })
            .ok_or("No storage root configured")?;
        // with_storage_root, not for_host: a read-only server must not run
        // the export worker or backup scheduler background writers.
        let core = Core::with_storage_root(storage_root);
        let backend = anansi_core::mcp_server::CoreMcpServerBackend::new(&core);
        let options = anansi_core::mcp_server::McpServerOptions {
            workspace_id,
            pairing_root,
        };
        eprintln!("ANANSI MCP server ready.");
        let mut stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();
        anansi_core::mcp_server::serve(&mut stdin, &mut stdout, &backend, &options)
            .map_err(|e| e.message)?;
        return Ok(());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let port = listener.local_addr()?.port();
    let session = Session::create(&root, port)?;
    let state = Arc::new(ApiState {
        core: Core::for_host(),
        token: session.token.clone(),
        host: format!("127.0.0.1:{port}"),
        origin: "http://127.0.0.1:1420".into(),
    });
    println!(
        "ANANSI local API on 127.0.0.1:{port}. Pairing file: {}",
        session.file.display()
    );
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    drop(session);
    Ok(())
}
