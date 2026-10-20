use anansi_api::{ApiState, pairing::Session, router};
use anansi_core::Core;
use std::{path::PathBuf, sync::Arc};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = 1451_u16;
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
