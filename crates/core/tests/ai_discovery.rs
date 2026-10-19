use anansi_core::Core;
use anansi_core::ai_discovery::{
    FitLevel, HardwareProfile, RuntimeKind, RuntimeStatus, generate_recommendations,
    inspect_hardware, probe_runtime,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn native_hardware_inspection_reports_truthful_resources() {
    let profile = inspect_hardware(None);
    assert!(!profile.architecture.is_empty());
    assert!(profile.logical_cpu_cores >= 1);
    assert!(profile.total_ram_bytes > 0);
    assert!(profile.available_ram_bytes > 0);
    assert!(profile.available_ram_bytes <= profile.total_ram_bytes);

    // If GPU is detected, ensure non-empty name and valid VRAM
    if let Some(gpu) = &profile.gpu_name {
        assert!(!gpu.trim().is_empty());
    }
}

#[test]
fn non_loopback_endpoints_fail_closed_as_unsupported() {
    // Master spec / security boundary: external hosts must never be probed
    for external in [
        "http://api.openai.com/v1",
        "http://192.168.1.100:11434",
        "https://127.0.0.1:11434",
        "ftp://localhost:11434",
        "http://cloud.example.org:8080",
    ] {
        let probe = probe_runtime(RuntimeKind::CustomLoopback, external);
        assert_eq!(
            probe.status,
            RuntimeStatus::Unsupported,
            "Endpoint {} should have been rejected as unsupported",
            external
        );
        assert!(probe.models.is_empty());
    }
}

#[test]
fn offline_loopback_runtime_returns_offline_without_hanging() {
    // Find an unused loopback port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Close it immediately

    let start = std::time::Instant::now();
    let probe = probe_runtime(RuntimeKind::Ollama, &format!("http://127.0.0.1:{}", port));
    let elapsed = start.elapsed();

    assert_eq!(probe.status, RuntimeStatus::Offline);
    assert!(probe.models.is_empty());
    assert!(
        elapsed < std::time::Duration::from_millis(2000),
        "Probe took too long: {:?}",
        elapsed
    );
}

#[test]
fn synthetic_ollama_runtime_probe_extracts_models_and_version() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("GET /api/tags HTTP/1.1"));

        let body = json!({
            "models": [
                {
                    "name": "llama3.1:8b",
                    "model": "llama3.1:8b",
                    "details": { "parameter_size": "8.0B" }
                },
                {
                    "name": "qwen2.5:7b",
                    "model": "qwen2.5:7b"
                }
            ],
            "version": "0.3.12"
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).unwrap();
    });

    let probe = probe_runtime(RuntimeKind::Ollama, &format!("http://127.0.0.1:{}", port));
    server_thread.join().unwrap();

    assert_eq!(probe.status, RuntimeStatus::Available);
    assert_eq!(probe.version, Some("0.3.12".to_string()));
    assert_eq!(
        probe.models,
        vec!["llama3.1:8b".to_string(), "qwen2.5:7b".to_string()]
    );
}

#[test]
fn synthetic_lm_studio_probe_extracts_models() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("GET /v1/models HTTP/1.1"));

        let body = json!({
            "data": [
                { "id": "meta-llama-3.1-8b-instruct" },
                { "id": "qwen2.5-14b-instruct" }
            ]
        })
        .to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).unwrap();
    });

    let probe = probe_runtime(RuntimeKind::LmStudio, &format!("http://127.0.0.1:{}", port));
    server_thread.join().unwrap();

    assert_eq!(probe.status, RuntimeStatus::Available);
    assert_eq!(
        probe.models,
        vec![
            "meta-llama-3.1-8b-instruct".to_string(),
            "qwen2.5-14b-instruct".to_string()
        ]
    );
}

#[test]
fn model_recommendations_scale_to_hardware_tiers() {
    let gb = 1024 * 1024 * 1024u64;

    // Constrained profile: 6 GB RAM
    let low_ram = HardwareProfile {
        architecture: "x86_64".into(),
        logical_cpu_cores: 4,
        total_ram_bytes: 6 * gb,
        available_ram_bytes: 4 * gb,
        free_storage_bytes: 100 * gb,
        gpu_name: None,
        vram_bytes: None,
    };
    let recs_low = generate_recommendations(&low_ram);
    let qwen_1_5b = recs_low
        .iter()
        .find(|r| r.model_tag == "qwen2.5:1.5b")
        .unwrap();
    assert_eq!(qwen_1_5b.fit_level, FitLevel::Recommended);
    let qwen_14b = recs_low
        .iter()
        .find(|r| r.model_tag == "qwen2.5:14b-instruct-q4_K_M")
        .unwrap();
    assert_eq!(qwen_14b.fit_level, FitLevel::Insufficient);

    // Standard profile: 12 GB RAM
    let mid_ram = HardwareProfile {
        architecture: "x86_64".into(),
        logical_cpu_cores: 8,
        total_ram_bytes: 12 * gb,
        available_ram_bytes: 9 * gb,
        free_storage_bytes: 200 * gb,
        gpu_name: None,
        vram_bytes: None,
    };
    let recs_mid = generate_recommendations(&mid_ram);
    let llama_8b = recs_mid
        .iter()
        .find(|r| r.model_tag == "llama3.1:8b-instruct-q4_K_M")
        .unwrap();
    assert_eq!(llama_8b.fit_level, FitLevel::Recommended);

    // High performance workstation: 32 GB RAM, 16 GB VRAM
    let high_ram = HardwareProfile {
        architecture: "x86_64".into(),
        logical_cpu_cores: 16,
        total_ram_bytes: 32 * gb,
        available_ram_bytes: 24 * gb,
        free_storage_bytes: 500 * gb,
        gpu_name: Some("NVIDIA GeForce RTX 4080".into()),
        vram_bytes: Some(16 * gb),
    };
    let recs_high = generate_recommendations(&high_ram);
    let qwen_14b_high = recs_high
        .iter()
        .find(|r| r.model_tag == "qwen2.5:14b-instruct-q4_K_M")
        .unwrap();
    assert_eq!(qwen_14b_high.fit_level, FitLevel::Recommended);
}

#[test]
fn core_ai_command_roundtrip_enforces_contract_and_consent() {
    let core = Core::default();

    // 1. Valid discovery
    let res = core
        .ai_command("discovery", json!({ "contractVersion": 1 }))
        .unwrap();
    assert_eq!(res["contractVersion"], 1);
    assert_eq!(res["downloadConsentRequired"], true);
    assert!(res["hardware"]["logicalCpuCores"].as_u64().unwrap() >= 1);
    assert!(res["runtimes"].as_array().unwrap().len() >= 2);
    assert!(!res["recommendations"].as_array().unwrap().is_empty());

    // 2. Unsupported contract version
    let bad_contract = core.ai_command("discovery", json!({ "contractVersion": 2 }));
    assert!(bad_contract.is_err());
    assert_eq!(
        bad_contract.unwrap_err().code,
        anansi_core::ErrorCode::UnsupportedContract
    );

    // 3. Unknown command
    let unknown = core.ai_command("unknown_op", json!({ "contractVersion": 1 }));
    assert!(unknown.is_err());
    assert_eq!(
        unknown.unwrap_err().code,
        anansi_core::ErrorCode::InvalidRequest
    );
}
