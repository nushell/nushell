//! eBPF program loading and management
//!
//! This module handles loading eBPF programs into the kernel using Aya,
//! and managing active probes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aya::maps::PerfEventArray;
use aya::programs::KProbe;
use aya::util::online_cpus;
use aya::Ebpf;
use bytes::BytesMut;
use thiserror::Error;

use crate::compiler::{CompileError, EbpfProgram, EbpfProgramType};

/// Errors that can occur during eBPF loading
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Compilation error: {0}")]
    Compile(#[from] CompileError),

    #[error("Failed to load eBPF program: {0}")]
    Load(String),

    #[error("Failed to attach probe: {0}")]
    Attach(String),

    #[error("Probe not found: {0}")]
    ProbeNotFound(u32),

    #[error("Unsupported program type: {0:?}")]
    UnsupportedProgramType(EbpfProgramType),

    #[error("Permission denied: eBPF requires CAP_BPF or root")]
    PermissionDenied,

    #[error("Program not found in ELF: {0}")]
    ProgramNotFound(String),

    #[error("Map not found: {0}")]
    MapNotFound(String),

    #[error("Perf buffer error: {0}")]
    PerfBuffer(String),
}

/// A perf buffer for one CPU
pub struct CpuPerfBuffer {
    cpu_id: u32,
    buf: aya::maps::perf::PerfEventArrayBuffer<aya::maps::MapData>,
}

/// Information about an active probe
pub struct ActiveProbe {
    /// Unique probe ID
    pub id: u32,
    /// The probe specification (e.g., "kprobe:sys_clone")
    pub probe_spec: String,
    /// When the probe was attached
    pub attached_at: Instant,
    /// The loaded eBPF object (keeps program alive)
    #[allow(dead_code)]
    ebpf: Ebpf,
    /// Whether this probe has a perf event map for output
    has_perf_map: bool,
    /// Perf buffers for each CPU (only if has_perf_map)
    perf_buffers: Vec<CpuPerfBuffer>,
}

impl std::fmt::Debug for ActiveProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveProbe")
            .field("id", &self.id)
            .field("probe_spec", &self.probe_spec)
            .field("attached_at", &self.attached_at)
            .field("has_perf_map", &self.has_perf_map)
            .finish()
    }
}

/// An event received from an eBPF program via bpf-emit
#[derive(Debug, Clone)]
pub struct BpfEvent {
    /// The value emitted by the eBPF program
    pub value: i64,
    /// Which CPU the event came from
    pub cpu: u32,
}

/// Global state for managing eBPF probes
pub struct EbpfState {
    /// Active probes indexed by ID
    probes: Mutex<HashMap<u32, ActiveProbe>>,
    /// Next probe ID
    next_id: AtomicU32,
}

impl Default for EbpfState {
    fn default() -> Self {
        Self::new()
    }
}

impl EbpfState {
    pub fn new() -> Self {
        Self {
            probes: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
        }
    }

    /// Get the next available probe ID
    fn next_probe_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Load and attach an eBPF program
    pub fn attach(&self, program: &EbpfProgram) -> Result<u32, LoadError> {
        // Generate ELF
        let elf_bytes = program.to_elf()?;

        // Load with Aya
        let mut ebpf = Ebpf::load(&elf_bytes).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("EPERM") || msg.contains("permission") {
                LoadError::PermissionDenied
            } else {
                LoadError::Load(msg)
            }
        })?;

        // Get the program by name
        let prog = ebpf
            .program_mut(&program.name)
            .ok_or_else(|| LoadError::ProgramNotFound(program.name.clone()))?;

        // Attach based on program type
        match program.prog_type {
            EbpfProgramType::Kprobe => {
                let kprobe: &mut KProbe = prog.try_into().map_err(|e| {
                    LoadError::Load(format!("Failed to convert to KProbe: {e}"))
                })?;
                kprobe
                    .load()
                    .map_err(|e| LoadError::Load(format!("Failed to load kprobe: {e}")))?;
                kprobe
                    .attach(&program.target, 0)
                    .map_err(|e| LoadError::Attach(format!("Failed to attach kprobe: {e}")))?;
            }
            other => return Err(LoadError::UnsupportedProgramType(other)),
        }

        // Check if program has a perf event map
        let has_perf_map = program.has_maps();
        let mut perf_buffers = Vec::new();

        // Set up perf buffers if the program uses bpf-emit
        if has_perf_map {
            let perf_array = ebpf
                .take_map("events")
                .ok_or_else(|| LoadError::MapNotFound("events".to_string()))?;

            let mut perf_array = PerfEventArray::try_from(perf_array)
                .map_err(|e| LoadError::PerfBuffer(format!("Failed to convert map: {e}")))?;

            // Open a buffer for each CPU
            let cpus = online_cpus()
                .map_err(|e| LoadError::PerfBuffer(format!("Failed to get CPUs: {e:?}")))?;

            for cpu_id in cpus {
                let buf = perf_array
                    .open(cpu_id, Some(64)) // 64 pages per buffer
                    .map_err(|e| {
                        LoadError::PerfBuffer(format!("Failed to open buffer for CPU {cpu_id}: {e}"))
                    })?;
                perf_buffers.push(CpuPerfBuffer { cpu_id, buf });
            }
        }

        // Store the active probe
        let id = self.next_probe_id();
        let probe_spec = format!("{}:{}", program.prog_type.section_prefix(), program.target);

        let active_probe = ActiveProbe {
            id,
            probe_spec,
            attached_at: Instant::now(),
            ebpf,
            has_perf_map,
            perf_buffers,
        };

        self.probes.lock().unwrap().insert(id, active_probe);

        Ok(id)
    }

    /// Poll for events from a probe's perf buffer
    ///
    /// Returns events emitted by the eBPF program via bpf-emit.
    /// The timeout specifies how long to wait for events.
    pub fn poll_events(&self, id: u32, _timeout: Duration) -> Result<Vec<BpfEvent>, LoadError> {
        let mut probes = self.probes.lock().unwrap();
        let probe = probes
            .get_mut(&id)
            .ok_or(LoadError::ProbeNotFound(id))?;

        if !probe.has_perf_map || probe.perf_buffers.is_empty() {
            // No perf map, return empty
            return Ok(Vec::new());
        }

        let mut events = Vec::new();

        // Read events from each pre-opened buffer
        // Use a buffer that can hold multiple 8-byte events
        let mut out_bufs: [BytesMut; 16] = std::array::from_fn(|_| BytesMut::with_capacity(64));

        for cpu_buf in &mut probe.perf_buffers {
            // Read available events (non-blocking)
            if let Ok(evts) = cpu_buf.buf.read_events(&mut out_bufs) {
                for out_buf in out_bufs.iter().take(evts.read) {
                    if out_buf.len() >= 8 {
                        let value = i64::from_le_bytes(out_buf[0..8].try_into().unwrap());
                        events.push(BpfEvent { value, cpu: cpu_buf.cpu_id });
                    }
                }
            }
        }

        Ok(events)
    }

    /// Detach a probe by ID
    pub fn detach(&self, id: u32) -> Result<(), LoadError> {
        let mut probes = self.probes.lock().unwrap();
        if probes.remove(&id).is_some() {
            // Dropping the ActiveProbe will detach the program
            Ok(())
        } else {
            Err(LoadError::ProbeNotFound(id))
        }
    }

    /// List all active probes
    pub fn list(&self) -> Vec<ProbeInfo> {
        let probes = self.probes.lock().unwrap();
        probes
            .values()
            .map(|p| ProbeInfo {
                id: p.id,
                probe_spec: p.probe_spec.clone(),
                uptime_secs: p.attached_at.elapsed().as_secs(),
            })
            .collect()
    }
}

/// Information about a probe for display
#[derive(Debug, Clone)]
pub struct ProbeInfo {
    pub id: u32,
    pub probe_spec: String,
    pub uptime_secs: u64,
}

/// Global eBPF state (lazily initialized)
static EBPF_STATE: std::sync::OnceLock<Arc<EbpfState>> = std::sync::OnceLock::new();

/// Get the global eBPF state
pub fn get_state() -> Arc<EbpfState> {
    EBPF_STATE
        .get_or_init(|| Arc::new(EbpfState::new()))
        .clone()
}

/// Parse a probe specification like "kprobe:sys_clone" or "tracepoint:syscalls/sys_enter_read"
pub fn parse_probe_spec(spec: &str) -> Result<(EbpfProgramType, String), LoadError> {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(LoadError::Load(format!(
            "Invalid probe spec: {spec}. Expected format: type:target (e.g., kprobe:sys_clone)"
        )));
    }

    let prog_type = match parts[0] {
        "kprobe" => EbpfProgramType::Kprobe,
        "kretprobe" => EbpfProgramType::Kretprobe,
        "tracepoint" => EbpfProgramType::Tracepoint,
        "raw_tracepoint" | "raw_tp" => EbpfProgramType::RawTracepoint,
        other => {
            return Err(LoadError::Load(format!(
                "Unknown probe type: {other}. Supported: kprobe, kretprobe, tracepoint, raw_tracepoint"
            )))
        }
    };

    Ok((prog_type, parts[1].to_string()))
}
