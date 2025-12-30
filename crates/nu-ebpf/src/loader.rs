//! eBPF program loading and management
//!
//! This module handles loading eBPF programs into the kernel using Aya,
//! and managing active probes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aya::maps::{HashMap as AyaHashMap, PerfEventArray};
use aya::programs::{KProbe, RawTracePoint, TracePoint};
use aya::util::online_cpus;
use aya::Ebpf;
use bytes::BytesMut;
use thiserror::Error;

use crate::compiler::{BpfFieldType, CompileError, EbpfProgram, EbpfProgramType, EventSchema};

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
    ebpf: Ebpf,
    /// Whether this probe has a perf event map for output
    has_perf_map: bool,
    /// Whether this probe has a counter hash map
    has_counter_map: bool,
    /// Perf buffers for each CPU (only if has_perf_map)
    perf_buffers: Vec<CpuPerfBuffer>,
    /// Optional schema for structured events
    event_schema: Option<EventSchema>,
}

impl std::fmt::Debug for ActiveProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveProbe")
            .field("id", &self.id)
            .field("probe_spec", &self.probe_spec)
            .field("attached_at", &self.attached_at)
            .field("has_perf_map", &self.has_perf_map)
            .field("has_counter_map", &self.has_counter_map)
            .field("event_schema", &self.event_schema.is_some())
            .finish()
    }
}

/// A field value in a structured event
#[derive(Debug, Clone)]
pub enum BpfFieldValue {
    /// An integer value
    Int(i64),
    /// A string value
    String(String),
}

/// The data payload of an eBPF event
#[derive(Debug, Clone)]
pub enum BpfEventData {
    /// An integer value (8 bytes from bpf-emit)
    Int(i64),
    /// A string value (16 bytes from bpf-emit-comm, null-terminated)
    String(String),
    /// Raw bytes for unknown sizes
    Bytes(Vec<u8>),
    /// A structured record with named fields
    Record(Vec<(String, BpfFieldValue)>),
}

/// An event received from an eBPF program via bpf-emit or bpf-emit-comm
#[derive(Debug, Clone)]
pub struct BpfEvent {
    /// The data emitted by the eBPF program
    pub data: BpfEventData,
    /// Which CPU the event came from
    pub cpu: u32,
}

/// A counter entry from the bpf-count hash map
#[derive(Debug, Clone)]
pub struct CounterEntry {
    /// The key (e.g., PID or comm as i64)
    pub key: i64,
    /// The count value
    pub count: i64,
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
            EbpfProgramType::Kretprobe => {
                // Kretprobe uses the same KProbe type - Aya detects it from the section name
                let kretprobe: &mut KProbe = prog.try_into().map_err(|e| {
                    LoadError::Load(format!("Failed to convert to KRetProbe: {e}"))
                })?;
                kretprobe
                    .load()
                    .map_err(|e| LoadError::Load(format!("Failed to load kretprobe: {e}")))?;
                kretprobe
                    .attach(&program.target, 0)
                    .map_err(|e| LoadError::Attach(format!("Failed to attach kretprobe: {e}")))?;
            }
            EbpfProgramType::Tracepoint => {
                // Tracepoint target format: "category/name" (e.g., "syscalls/sys_enter_openat")
                let parts: Vec<&str> = program.target.splitn(2, '/').collect();
                if parts.len() != 2 {
                    return Err(LoadError::Load(format!(
                        "Invalid tracepoint target: {}. Expected format: category/name",
                        program.target
                    )));
                }
                let (category, name) = (parts[0], parts[1]);

                let tracepoint: &mut TracePoint = prog.try_into().map_err(|e| {
                    LoadError::Load(format!("Failed to convert to TracePoint: {e}"))
                })?;
                tracepoint
                    .load()
                    .map_err(|e| LoadError::Load(format!("Failed to load tracepoint: {e}")))?;
                tracepoint
                    .attach(category, name)
                    .map_err(|e| LoadError::Attach(format!("Failed to attach tracepoint: {e}")))?;
            }
            EbpfProgramType::RawTracepoint => {
                // Raw tracepoint target is just the name (e.g., "sys_enter")
                let raw_tp: &mut RawTracePoint = prog.try_into().map_err(|e| {
                    LoadError::Load(format!("Failed to convert to RawTracePoint: {e}"))
                })?;
                raw_tp
                    .load()
                    .map_err(|e| LoadError::Load(format!("Failed to load raw_tracepoint: {e}")))?;
                raw_tp
                    .attach(&program.target)
                    .map_err(|e| LoadError::Attach(format!("Failed to attach raw_tracepoint: {e}")))?;
            }
            other => return Err(LoadError::UnsupportedProgramType(other)),
        }

        // Check for maps
        let has_perf_map = ebpf.map("events").is_some();
        let has_counter_map = ebpf.map("counters").is_some();
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
            has_counter_map,
            perf_buffers,
            event_schema: program.event_schema.clone(),
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

        // Clone the schema for use in parsing (to avoid borrow issues)
        let schema = probe.event_schema.clone();

        // Read events from each pre-opened buffer
        let mut out_bufs: [BytesMut; 16] = std::array::from_fn(|_| BytesMut::with_capacity(256));

        for cpu_buf in &mut probe.perf_buffers {
            // Read available events (non-blocking)
            if let Ok(evts) = cpu_buf.buf.read_events(&mut out_bufs) {
                for out_buf in out_bufs.iter().take(evts.read) {
                    let data = if let Some(ref event_schema) = schema {
                        // We have a schema - deserialize structured event
                        Self::deserialize_structured_event(out_buf, event_schema)
                    } else {
                        // No schema - use legacy size-based detection
                        Self::deserialize_simple_event(out_buf)
                    };

                    if let Some(data) = data {
                        events.push(BpfEvent { data, cpu: cpu_buf.cpu_id });
                    }
                }
            }
        }

        Ok(events)
    }

    /// Deserialize a simple (non-structured) event based on size
    fn deserialize_simple_event(buf: &[u8]) -> Option<BpfEventData> {
        // Perf buffer may add padding, so we use size ranges
        // - 8-15 bytes: integer from bpf-emit
        // - 16+ bytes: string (bpf-emit-comm uses 16, bpf-read-str uses 128)
        if buf.len() >= 8 && buf.len() < 16 {
            // 8-15 bytes = integer from bpf-emit (may have padding)
            let value = i64::from_le_bytes(buf[0..8].try_into().unwrap());
            Some(BpfEventData::Int(value))
        } else if buf.len() >= 16 {
            // 16+ bytes = string (from bpf-emit-comm or bpf-read-str)
            // Find null terminator within the buffer
            let null_pos = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            let s = String::from_utf8_lossy(&buf[..null_pos]).to_string();
            Some(BpfEventData::String(s))
        } else if !buf.is_empty() {
            // Unknown size - return raw bytes
            Some(BpfEventData::Bytes(buf.to_vec()))
        } else {
            None
        }
    }

    /// Deserialize a structured event using the schema
    fn deserialize_structured_event(buf: &[u8], schema: &EventSchema) -> Option<BpfEventData> {
        if buf.len() < schema.total_size {
            // Buffer too small for the expected schema
            return Self::deserialize_simple_event(buf);
        }

        let mut fields = Vec::with_capacity(schema.fields.len());

        for field in &schema.fields {
            let field_buf = &buf[field.offset..];
            let value = match field.field_type {
                BpfFieldType::Int => {
                    if field_buf.len() >= 8 {
                        let val = i64::from_le_bytes(field_buf[0..8].try_into().unwrap());
                        BpfFieldValue::Int(val)
                    } else {
                        BpfFieldValue::Int(0)
                    }
                }
                BpfFieldType::Comm => {
                    // 16-byte comm string
                    let max_len = field_buf.len().min(16);
                    let null_pos = field_buf[..max_len].iter().position(|&b| b == 0).unwrap_or(max_len);
                    let s = String::from_utf8_lossy(&field_buf[..null_pos]).to_string();
                    BpfFieldValue::String(s)
                }
                BpfFieldType::String => {
                    // 128-byte string
                    let max_len = field_buf.len().min(128);
                    let null_pos = field_buf[..max_len].iter().position(|&b| b == 0).unwrap_or(max_len);
                    let s = String::from_utf8_lossy(&field_buf[..null_pos]).to_string();
                    BpfFieldValue::String(s)
                }
            };
            fields.push((field.name.clone(), value));
        }

        Some(BpfEventData::Record(fields))
    }

    /// Read all counter entries from a probe's counter map
    ///
    /// Returns all key-value pairs from the bpf-count hash map.
    pub fn get_counters(&self, id: u32) -> Result<Vec<CounterEntry>, LoadError> {
        let mut probes = self.probes.lock().unwrap();
        let probe = probes
            .get_mut(&id)
            .ok_or(LoadError::ProbeNotFound(id))?;

        if !probe.has_counter_map {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();

        // Get the counter map
        if let Some(map) = probe.ebpf.map_mut("counters") {
            let counter_map: AyaHashMap<_, i64, i64> = AyaHashMap::try_from(map)
                .map_err(|e| LoadError::MapNotFound(format!("Failed to convert counters map: {e}")))?;

            // Iterate over all entries
            for item in counter_map.iter() {
                if let Ok((key, count)) = item {
                    entries.push(CounterEntry { key, count });
                }
            }
        }

        Ok(entries)
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
