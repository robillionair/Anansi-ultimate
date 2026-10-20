use crate::{CommandError, Core, ErrorCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeKind {
    Ollama,
    LmStudio,
    CustomLoopback,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeStatus {
    Available,
    Offline,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FitLevel {
    Recommended,
    Comfortable,
    Tight,
    Insufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveredRuntime {
    pub kind: RuntimeKind,
    pub endpoint: String,
    pub status: RuntimeStatus,
    pub models: Vec<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRecommendation {
    pub model_tag: String,
    pub parameter_size: String,
    pub quantization: String,
    pub minimum_ram_bytes: u64,
    pub description: String,
    pub fit_level: FitLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareProfile {
    pub architecture: String,
    pub logical_cpu_cores: u32,
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub free_storage_bytes: u64,
    pub gpu_name: Option<String>,
    pub vram_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiDiscoveryRequest {
    pub contract_version: u8,
    pub custom_endpoints: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiDiscoveryResponse {
    pub contract_version: u8,
    pub hardware: HardwareProfile,
    pub runtimes: Vec<DiscoveredRuntime>,
    pub recommendations: Vec<ModelRecommendation>,
    pub download_consent_required: bool,
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    struct MEMORYSTATUSEX {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> i32;
        fn GetDiskFreeSpaceExW(
            lp_directory_name: *const u16,
            lp_free_bytes_available: *mut u64,
            lp_total_number_of_bytes: *mut u64,
            lp_total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            h_key: usize,
            lp_sub_key: *const u16,
            ul_options: u32,
            sam_desired: u32,
            phk_result: *mut usize,
        ) -> i32;
        fn RegQueryValueExW(
            h_key: usize,
            lp_value_name: *const u16,
            lp_reserved: *mut u32,
            lp_type: *mut u32,
            lp_data: *mut u8,
            lpcb_data: *mut u32,
        ) -> i32;
        fn RegCloseKey(h_key: usize) -> i32;
    }

    pub fn memory_stats() -> (u64, u64) {
        let mut status = MEMORYSTATUSEX {
            dw_length: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok != 0 {
            (status.ull_total_phys, status.ull_avail_phys)
        } else {
            (0, 0)
        }
    }

    pub fn free_storage(path: Option<&Path>) -> u64 {
        let dir = path.unwrap_or_else(|| Path::new("."));
        let mut wide: Vec<u16> = dir.as_os_str().encode_wide().collect();
        wide.push(0);
        let mut free_bytes_avail: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_avail,
                &mut total_bytes,
                &mut total_free_bytes,
            )
        };
        if ok != 0 { free_bytes_avail } else { 0 }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        let mut wide: Vec<u16> = s.encode_utf16().collect();
        wide.push(0);
        wide
    }

    pub fn inspect_gpu() -> (Option<String>, Option<u64>) {
        const HKEY_LOCAL_MACHINE: usize = 0x8000_0002;
        const KEY_READ: u32 = 0x20019;

        let mut best_gpu: Option<String> = None;
        let mut best_vram: Option<u64> = None;

        // Inspect display adapter registry class
        for index in 0..8 {
            let subkey = format!(
                "SYSTEM\\CurrentControlSet\\Control\\Class\\{{4d36e968-e325-11ce-bfc1-08002be10318}}\\{:04}",
                index
            );
            let wide_subkey = to_wide(&subkey);
            let mut hkey: usize = 0;
            let status = unsafe {
                RegOpenKeyExW(
                    HKEY_LOCAL_MACHINE,
                    wide_subkey.as_ptr(),
                    0,
                    KEY_READ,
                    &mut hkey,
                )
            };
            if status != 0 {
                continue;
            }

            // Read DriverDesc
            let desc_wide = to_wide("DriverDesc");
            let mut desc_buf = [0u8; 512];
            let mut desc_len = desc_buf.len() as u32;
            let mut val_type: u32 = 0;
            let desc_status = unsafe {
                RegQueryValueExW(
                    hkey,
                    desc_wide.as_ptr(),
                    std::ptr::null_mut(),
                    &mut val_type,
                    desc_buf.as_mut_ptr(),
                    &mut desc_len,
                )
            };

            let mut gpu_name = None;
            if desc_status == 0 && desc_len > 1 {
                let u16_slice = unsafe {
                    std::slice::from_raw_parts(
                        desc_buf.as_ptr() as *const u16,
                        (desc_len as usize) / 2,
                    )
                };
                let name = String::from_utf16_lossy(u16_slice)
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    gpu_name = Some(name);
                }
            }

            // Read HardwareInformation.qwMemorySize (64-bit) or MemorySize (32-bit)
            let mut vram: Option<u64> = None;
            let qw_mem_wide = to_wide("HardwareInformation.qwMemorySize");
            let mut qw_val: u64 = 0;
            let mut qw_len = std::mem::size_of::<u64>() as u32;
            let qw_status = unsafe {
                RegQueryValueExW(
                    hkey,
                    qw_mem_wide.as_ptr(),
                    std::ptr::null_mut(),
                    &mut val_type,
                    &mut qw_val as *mut u64 as *mut u8,
                    &mut qw_len,
                )
            };
            if qw_status == 0 && qw_val > 0 {
                vram = Some(qw_val);
            } else {
                let dw_mem_wide = to_wide("HardwareInformation.MemorySize");
                let mut dw_val: u32 = 0;
                let mut dw_len = std::mem::size_of::<u32>() as u32;
                let dw_status = unsafe {
                    RegQueryValueExW(
                        hkey,
                        dw_mem_wide.as_ptr(),
                        std::ptr::null_mut(),
                        &mut val_type,
                        &mut dw_val as *mut u32 as *mut u8,
                        &mut dw_len,
                    )
                };
                if dw_status == 0 && dw_val > 0 {
                    vram = Some(dw_val as u64);
                }
            }

            unsafe {
                RegCloseKey(hkey);
            }

            if let Some(name) = gpu_name {
                // Prefer dedicated/discrete GPUs (e.g. NVIDIA, AMD, Radeon, GeForce) or higher VRAM
                let is_discrete = name.contains("NVIDIA")
                    || name.contains("GeForce")
                    || name.contains("Radeon")
                    || name.contains("AMD");
                let cur_vram = vram.unwrap_or(0);
                let prev_vram = best_vram.unwrap_or(0);

                if best_gpu.is_none() || is_discrete || cur_vram > prev_vram {
                    best_gpu = Some(name);
                    best_vram = vram;
                }
            }
        }

        (best_gpu, best_vram)
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    pub fn memory_stats() -> (u64, u64) {
        // Fallback for non-windows platforms in test/dev
        (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
    }

    pub fn free_storage(_path: Option<&Path>) -> u64 {
        50 * 1024 * 1024 * 1024
    }

    pub fn inspect_gpu() -> (Option<String>, Option<u64>) {
        (None, None)
    }
}

pub fn inspect_hardware(storage_root: Option<&std::path::Path>) -> HardwareProfile {
    let architecture = std::env::consts::ARCH.to_string();
    let logical_cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    let (total_ram_bytes, available_ram_bytes) = platform::memory_stats();
    let free_storage_bytes = platform::free_storage(storage_root);
    let (gpu_name, vram_bytes) = platform::inspect_gpu();

    HardwareProfile {
        architecture,
        logical_cpu_cores,
        total_ram_bytes,
        available_ram_bytes,
        free_storage_bytes,
        gpu_name,
        vram_bytes,
    }
}

pub fn generate_recommendations(profile: &HardwareProfile) -> Vec<ModelRecommendation> {
    let effective_capacity = profile
        .vram_bytes
        .map(|v| v.max(profile.total_ram_bytes))
        .unwrap_or(profile.total_ram_bytes);

    let gb = 1024 * 1024 * 1024u64;

    let mut recs = Vec::new();

    // 1. Qwen 2.5 1.5B (Compact / Ultra-lightweight)
    let min_ram_1_5b = 2 * gb;
    let fit_1_5b = if effective_capacity < min_ram_1_5b {
        FitLevel::Insufficient
    } else if profile.available_ram_bytes < min_ram_1_5b {
        FitLevel::Tight
    } else if effective_capacity < 8 * gb {
        FitLevel::Recommended
    } else {
        FitLevel::Comfortable
    };
    recs.push(ModelRecommendation {
        model_tag: "qwen2.5:1.5b".to_string(),
        parameter_size: "1.5B".to_string(),
        quantization: "Q4_K_M".to_string(),
        minimum_ram_bytes: min_ram_1_5b,
        description: "Ultra-lightweight model for constrained CPU-only or low-memory environments."
            .to_string(),
        fit_level: fit_1_5b,
    });

    // 2. Llama 3.2 3B (Fast local utility)
    let min_ram_3b = 4 * gb;
    let fit_3b = if effective_capacity < min_ram_3b {
        FitLevel::Insufficient
    } else if profile.available_ram_bytes < min_ram_3b {
        FitLevel::Tight
    } else if effective_capacity < 8 * gb {
        FitLevel::Recommended
    } else {
        FitLevel::Comfortable
    };
    recs.push(ModelRecommendation {
        model_tag: "llama3.2:3b".to_string(),
        parameter_size: "3B".to_string(),
        quantization: "Q4_K_M".to_string(),
        minimum_ram_bytes: min_ram_3b,
        description: "Compact low-latency model suitable for lightweight CPU and resource-constrained systems."
            .to_string(),
        fit_level: fit_3b,
    });

    // 3. Llama 3.1 8B Instruct (Standard balanced reasoning)
    let min_ram_8b = 8 * gb;
    let fit_8b = if effective_capacity < min_ram_8b {
        FitLevel::Insufficient
    } else if profile.available_ram_bytes < min_ram_8b {
        FitLevel::Tight
    } else if effective_capacity >= 8 * gb && effective_capacity < 16 * gb {
        FitLevel::Recommended
    } else {
        FitLevel::Comfortable
    };
    recs.push(ModelRecommendation {
        model_tag: "llama3.1:8b-instruct-q4_K_M".to_string(),
        parameter_size: "8B".to_string(),
        quantization: "Q4_K_M".to_string(),
        minimum_ram_bytes: min_ram_8b,
        description: "Balanced general reasoning, code synthesis, and high fidelity memory retrieval grounding."
            .to_string(),
        fit_level: fit_8b,
    });

    // 4. Qwen 2.5 7B Instruct (Strong structured & multi-lingual synthesis)
    let min_ram_7b = 8 * gb;
    let fit_7b = if effective_capacity < min_ram_7b {
        FitLevel::Insufficient
    } else if profile.available_ram_bytes < min_ram_7b {
        FitLevel::Tight
    } else if effective_capacity >= 8 * gb && effective_capacity < 16 * gb {
        FitLevel::Recommended
    } else {
        FitLevel::Comfortable
    };
    recs.push(ModelRecommendation {
        model_tag: "qwen2.5:7b-instruct-q4_K_M".to_string(),
        parameter_size: "7B".to_string(),
        quantization: "Q4_K_M".to_string(),
        minimum_ram_bytes: min_ram_7b,
        description:
            "High-performance reasoning and structured knowledge synthesis with extensive context."
                .to_string(),
        fit_level: fit_7b,
    });

    // 5. Qwen 2.5 14B Instruct (Deep multi-hop reasoning & complex context)
    let min_ram_14b = 14 * gb;
    let fit_14b = if effective_capacity < min_ram_14b {
        FitLevel::Insufficient
    } else if profile.available_ram_bytes < min_ram_14b {
        FitLevel::Tight
    } else {
        FitLevel::Recommended
    };
    recs.push(ModelRecommendation {
        model_tag: "qwen2.5:14b-instruct-q4_K_M".to_string(),
        parameter_size: "14B".to_string(),
        quantization: "Q4_K_M".to_string(),
        minimum_ram_bytes: min_ram_14b,
        description: "Advanced multi-hop reasoning and deep contextual analysis for workstations with 16GB+ RAM/VRAM."
            .to_string(),
        fit_level: fit_14b,
    });

    recs
}

fn parse_loopback_url(endpoint: &str) -> Option<(String, u16, String)> {
    let trimmed = endpoint.trim();
    let without_proto = trimmed.strip_prefix("http://")?;

    let (host_port, path) = match without_proto.find('/') {
        Some(idx) => (&without_proto[..idx], &without_proto[idx..]),
        None => (without_proto, "/"),
    };

    let (host, port) = if let Some(idx) = host_port.find(':') {
        let h = &host_port[..idx];
        let p: u16 = host_port[idx + 1..].parse().ok()?;
        (h, p)
    } else {
        (host_port, 80)
    };

    // Strict security constraint: only allow loopback addresses to prevent scanning external networks
    if host != "127.0.0.1" && host != "localhost" && host != "[::1]" {
        return None;
    }

    Some((host.to_string(), port, path.to_string()))
}

fn probe_http_loopback(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
) -> Result<(u16, String), ()> {
    let addr_str = format!("{}:{}", host, port);
    let socket_addrs: Vec<SocketAddr> = addr_str.to_socket_addrs().map_err(|_| ())?.collect();
    if socket_addrs.is_empty() {
        return Err(());
    }

    let mut stream = TcpStream::connect_timeout(&socket_addrs[0], timeout).map_err(|_| ())?;
    stream.set_read_timeout(Some(timeout)).map_err(|_| ())?;
    stream.set_write_timeout(Some(timeout)).map_err(|_| ())?;

    use std::io::{Read, Write};
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: ANANSI/Discovery\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        path, host, port
    );
    stream.write_all(request.as_bytes()).map_err(|_| ())?;

    let mut response_buf = Vec::with_capacity(32 * 1024);
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                response_buf.extend_from_slice(&chunk[..n]);
                if response_buf.len() > 64 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let text = String::from_utf8_lossy(&response_buf);
    let mut parts = text.splitn(2, "\r\n\r\n");
    let header_part = parts.next().unwrap_or("");
    let body_part = parts.next().unwrap_or("").to_string();

    let status_code: u16 = header_part
        .lines()
        .next()
        .and_then(|line| {
            let mut words = line.split_whitespace();
            words.next()?; // HTTP/1.1
            words.next()?.parse().ok()
        })
        .unwrap_or(0);

    Ok((status_code, body_part))
}

pub fn probe_runtime(kind: RuntimeKind, endpoint: &str) -> DiscoveredRuntime {
    let (default_path, kind) = match kind {
        RuntimeKind::Ollama => ("/api/tags", RuntimeKind::Ollama),
        RuntimeKind::LmStudio => ("/v1/models", RuntimeKind::LmStudio),
        RuntimeKind::CustomLoopback => ("/v1/models", RuntimeKind::CustomLoopback),
    };

    let Some((host, port, req_path)) = parse_loopback_url(endpoint) else {
        return DiscoveredRuntime {
            kind,
            endpoint: endpoint.to_string(),
            status: RuntimeStatus::Unsupported,
            models: Vec::new(),
            version: None,
        };
    };

    let path_to_probe = if req_path == "/" {
        default_path
    } else {
        &req_path
    };

    let probe_result = probe_http_loopback(&host, port, path_to_probe, Duration::from_millis(1500));

    match probe_result {
        Ok((200, body)) => {
            let mut models = Vec::new();
            let mut version = None;

            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                match kind {
                    RuntimeKind::Ollama => {
                        if let Some(arr) = parsed.get("models").and_then(|v| v.as_array()) {
                            for m in arr {
                                if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                                    models.push(name.to_string());
                                } else if let Some(model) = m.get("model").and_then(|v| v.as_str())
                                {
                                    models.push(model.to_string());
                                }
                            }
                        }
                    }
                    RuntimeKind::LmStudio | RuntimeKind::CustomLoopback => {
                        if let Some(arr) = parsed.get("data").and_then(|v| v.as_array()) {
                            for m in arr {
                                if let Some(id) = m.get("id").and_then(|v| v.as_str()) {
                                    models.push(id.to_string());
                                }
                            }
                        }
                    }
                }
                if let Some(ver) = parsed.get("version").and_then(|v| v.as_str()) {
                    version = Some(ver.to_string());
                }
            }

            models.sort();
            models.dedup();

            DiscoveredRuntime {
                kind,
                endpoint: endpoint.to_string(),
                status: RuntimeStatus::Available,
                models,
                version,
            }
        }
        Ok((_, _)) => DiscoveredRuntime {
            kind,
            endpoint: endpoint.to_string(),
            status: RuntimeStatus::Unsupported,
            models: Vec::new(),
            version: None,
        },
        Err(_) => DiscoveredRuntime {
            kind,
            endpoint: endpoint.to_string(),
            status: RuntimeStatus::Offline,
            models: Vec::new(),
            version: None,
        },
    }
}

pub fn discover_all_runtimes(custom_endpoints: Option<&[String]>) -> Vec<DiscoveredRuntime> {
    let mut runtimes = Vec::new();

    // Default 1: Ollama
    runtimes.push(probe_runtime(RuntimeKind::Ollama, "http://127.0.0.1:11434"));

    // Default 2: LM Studio
    runtimes.push(probe_runtime(
        RuntimeKind::LmStudio,
        "http://127.0.0.1:1234",
    ));

    // Custom loopback endpoints if supplied
    if let Some(custom) = custom_endpoints {
        for ep in custom {
            runtimes.push(probe_runtime(RuntimeKind::CustomLoopback, ep));
        }
    }

    runtimes
}

impl Core {
    pub fn ai_discovery(
        &self,
        request: AiDiscoveryRequest,
    ) -> Result<AiDiscoveryResponse, CommandError> {
        if request.contract_version != 1 {
            return Err(CommandError::new(
                ErrorCode::UnsupportedContract,
                "This contract version is not supported.",
            ));
        }

        let storage_root = self.storage_root().ok();
        let hardware = inspect_hardware(storage_root.as_deref());
        let recommendations = generate_recommendations(&hardware);
        let runtimes = discover_all_runtimes(request.custom_endpoints.as_deref());

        Ok(AiDiscoveryResponse {
            contract_version: 1,
            hardware,
            runtimes,
            recommendations,
            download_consent_required: true,
        })
    }

    pub fn ai_command(
        &self,
        operation: &str,
        value: serde_json::Value,
    ) -> Result<serde_json::Value, CommandError> {
        match operation {
            "ask-start" => {
                let request = serde_json::from_value(value).map_err(|_| {
                    CommandError::new(ErrorCode::InvalidRequest, "Use a supported Ask request.")
                })?;
                serde_json::to_value(self.start_ask(request)?)
                    .map_err(|_| crate::storage::unavailable())
            }
            "ask-poll" | "ask-cancel" | "ask-forget" => {
                let request = serde_json::from_value(value).map_err(|_| {
                    CommandError::new(
                        ErrorCode::InvalidRequest,
                        "Use a supported Ask run request.",
                    )
                })?;
                serde_json::to_value(self.control_ask(operation, request)?)
                    .map_err(|_| crate::storage::unavailable())
            }
            "discovery" => {
                let req: AiDiscoveryRequest = serde_json::from_value(value).map_err(|_| {
                    CommandError::new(
                        ErrorCode::InvalidRequest,
                        "Use the supported AI discovery request.",
                    )
                })?;
                let response = self.ai_discovery(req)?;
                serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
            }
            "ask" => {
                let req: crate::ask::AskRequest = serde_json::from_value(value).map_err(|_| {
                    CommandError::new(
                        ErrorCode::InvalidRequest,
                        "Use the supported Ask ANANSI request.",
                    )
                })?;
                let response = self.ask_anansi(req)?;
                serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
            }
            "hybrid-search" => {
                let req: crate::hybrid::HybridSearchRequest = serde_json::from_value(value)
                    .map_err(|_| {
                        CommandError::new(
                            ErrorCode::InvalidRequest,
                            "Use the supported hybrid search request.",
                        )
                    })?;
                let storage = self.storage()?;
                let conn = storage
                    .connection(false)?
                    .ok_or_else(crate::storage::unavailable)?;
                let response = crate::hybrid::hybrid_search_with_status(
                    &conn,
                    Some(storage.root_path()),
                    &req,
                )?;
                serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
            }
            "context" => {
                let req: crate::context::ContextCompilationRequest = serde_json::from_value(value)
                    .map_err(|_| {
                        CommandError::new(
                            ErrorCode::InvalidRequest,
                            "Use the supported context compilation request.",
                        )
                    })?;
                let storage = self.storage()?;
                let conn = storage
                    .connection(false)?
                    .ok_or_else(crate::storage::unavailable)?;
                let root = storage.root_path();
                let response = crate::context::compile_context(&conn, root, &req)?;
                serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
            }
            "index-embeddings" => {
                let req: crate::semantic::IndexEmbeddingsRequest = serde_json::from_value(value)
                    .map_err(|_| {
                        CommandError::new(
                            ErrorCode::InvalidRequest,
                            "Use the supported index embeddings request.",
                        )
                    })?;
                if req.contract_version != 1 {
                    return Err(CommandError::new(
                        ErrorCode::UnsupportedContract,
                        "This contract version is not supported.",
                    ));
                }
                let storage = self.storage()?;
                let conn = storage
                    .connection(false)?
                    .ok_or_else(crate::storage::unavailable)?;
                let root = storage.root_path();
                let model = req
                    .model
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(crate::semantic::DEFAULT_EMBEDDING_MODEL)
                    .to_owned();
                let endpoint = req.endpoint.as_deref();

                let (indexed_memories, indexed_chunks, dimension) =
                    if let Some(mem_id) = &req.memory_id {
                        let (chunks, dim) = crate::semantic::embed_memory_with_hook(
                            &conn,
                            root,
                            &req.workspace_id,
                            mem_id,
                            &model,
                            endpoint,
                            || Ok(()),
                        )?;
                        (usize::from(chunks > 0), chunks, dim)
                    } else {
                        crate::semantic::embed_workspace_memories_detailed(
                            &conn,
                            root,
                            &req.workspace_id,
                            &model,
                            endpoint,
                        )?
                    };

                let response = crate::semantic::IndexEmbeddingsResponse {
                    contract_version: 1,
                    indexed_memories,
                    indexed_chunks,
                    model,
                    dimension,
                };
                serde_json::to_value(response).map_err(|_| crate::storage::unavailable())
            }
            _ => Err(CommandError::new(
                ErrorCode::InvalidRequest,
                "Use a supported AI command.",
            )),
        }
    }
}
