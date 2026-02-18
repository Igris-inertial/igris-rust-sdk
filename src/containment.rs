//! Containment bounds configuration and violation observability for Igris Runtime.
//!
//! ## Deterministic failure semantics
//!
//! - **Time violation**: Worker process exceeded `max_tick_ms`. The supervisor sends
//!   SIGKILL, appends a signed [`ViolationRecord`] to the audit log, and respawns a
//!   fresh worker. The SDK captures this via [`Runtime::on_violation`] or
//!   [`Runtime::get_last_violation`].
//! - **CPU violation**: Worker exceeded the cgroup CPU quota. Treated identically.
//! - **Cloud outage**: `Runtime::chat` falls back to local when `auto_fallback` is
//!   true; returns `Err(IgrisError::Network(_))` otherwise.
//! - **Worker SIGKILL**: Supervisor reaps the process, destroys the cgroup, records
//!   the violation, and spawns a fresh worker automatically.

use serde::{Deserialize, Serialize};

/// Kind of containment violation emitted by the Igris Runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationKind {
    Time,
    Cpu,
}

/// Parsed containment violation record from the Igris Runtime audit log.
///
/// The runtime writes one JSONL line per violation; this struct mirrors that
/// structure so SDK consumers can inspect, log, or forward records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationRecord {
    /// UUID v7 identifier.
    pub id: String,
    /// ISO-8601 UTC timestamp.
    pub timestamp: String,
    pub violation_kind: ViolationKind,
    pub context: serde_json::Value,
    pub previous_hash: String,
    pub hash: String,
    /// Base64-encoded Ed25519 signature over `hash`.
    pub signature: String,
}

/// Containment bounds passed to the Igris Runtime.
///
/// These mirror the `Bounds` struct inside `igris-safety` and control the CPU
/// quota and per-tick time limit enforced on each inference worker process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    /// Maximum CPU usage as a percentage (1–100). Enforced via Linux cgroups.
    pub cpu_percent: u8,
    /// Maximum RSS memory in megabytes (> 0).
    pub memory_mb: u32,
    /// Hard deadline in milliseconds for each worker execution.
    pub max_tick_ms: u64,
}

impl Default for Bounds {
    fn default() -> Self {
        Self { cpu_percent: 80, memory_mb: 512, max_tick_ms: 200 }
    }
}

impl Bounds {
    /// Create bounds with validation.
    ///
    /// # Errors
    /// Returns `Err` if any field is out of the valid range.
    pub fn new(cpu_percent: u8, memory_mb: u32, max_tick_ms: u64) -> Result<Self, String> {
        if cpu_percent == 0 || cpu_percent > 100 {
            return Err(format!("cpu_percent must be 1–100, got {}", cpu_percent));
        }
        if memory_mb == 0 {
            return Err("memory_mb must be > 0".to_string());
        }
        if max_tick_ms == 0 {
            return Err("max_tick_ms must be > 0".to_string());
        }
        Ok(Self { cpu_percent, memory_mb, max_tick_ms })
    }

    /// Serialise to wire format for the `X-Igris-Bounds` request header.
    pub fn to_header_value(&self) -> String {
        serde_json::json!({
            "cpu_percent": self.cpu_percent,
            "memory_mb": self.memory_mb,
            "max_tick_ms": self.max_tick_ms,
        })
        .to_string()
    }

    /// Return env-var name→value pairs for propagating bounds to a subprocess.
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("IGRIS_MAX_CPU_PERCENT".to_string(), self.cpu_percent.to_string()),
            ("IGRIS_MAX_MEMORY_MB".to_string(), self.memory_mb.to_string()),
            ("IGRIS_MAX_TICK_MS".to_string(), self.max_tick_ms.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_defaults() {
        let b = Bounds::default();
        assert_eq!(b.cpu_percent, 80);
        assert_eq!(b.memory_mb, 512);
        assert_eq!(b.max_tick_ms, 200);
    }

    #[test]
    fn test_bounds_new_valid() {
        let b = Bounds::new(50, 256, 100).unwrap();
        assert_eq!(b.cpu_percent, 50);
        assert_eq!(b.memory_mb, 256);
        assert_eq!(b.max_tick_ms, 100);
    }

    #[test]
    fn test_bounds_cpu_zero_rejected() {
        assert!(Bounds::new(0, 512, 200).is_err());
    }

    #[test]
    fn test_bounds_cpu_over_100_rejected() {
        assert!(Bounds::new(101, 512, 200).is_err());
    }

    #[test]
    fn test_bounds_memory_zero_rejected() {
        assert!(Bounds::new(80, 0, 200).is_err());
    }

    #[test]
    fn test_bounds_tick_zero_rejected() {
        assert!(Bounds::new(80, 512, 0).is_err());
    }

    #[test]
    fn test_header_value_is_valid_json() {
        let b = Bounds::new(40, 256, 150).unwrap();
        let h = b.to_header_value();
        let v: serde_json::Value = serde_json::from_str(&h).unwrap();
        assert_eq!(v["cpu_percent"], 40);
        assert_eq!(v["memory_mb"], 256);
        assert_eq!(v["max_tick_ms"], 150);
    }

    #[test]
    fn test_env_vars() {
        let b = Bounds::new(60, 512, 200).unwrap();
        let ev: std::collections::HashMap<_, _> = b.to_env_vars().into_iter().collect();
        assert_eq!(ev["IGRIS_MAX_CPU_PERCENT"], "60");
        assert_eq!(ev["IGRIS_MAX_MEMORY_MB"], "512");
        assert_eq!(ev["IGRIS_MAX_TICK_MS"], "200");
    }

    #[test]
    fn test_violation_record_deserialize() {
        let json = r#"{
            "id": "0191f4a0-0000-7000-8000-000000000001",
            "timestamp": "2026-02-18T09:00:00Z",
            "violation_kind": "Time",
            "context": {"task": "chat"},
            "previous_hash": "",
            "hash": "deadbeefdeadbeef",
            "signature": "c2lnbmF0dXJl"
        }"#;
        let r: ViolationRecord = serde_json::from_str(json).unwrap();
        assert_eq!(r.violation_kind, ViolationKind::Time);
        assert_eq!(r.hash, "deadbeefdeadbeef");
        assert!(!r.signature.is_empty());
    }

    #[test]
    fn test_violation_kind_cpu_deserialize() {
        let json = r#"{
            "id": "id",
            "timestamp": "2026-02-18T09:00:00Z",
            "violation_kind": "Cpu",
            "context": {},
            "previous_hash": "",
            "hash": "abc",
            "signature": "sig"
        }"#;
        let r: ViolationRecord = serde_json::from_str(json).unwrap();
        assert_eq!(r.violation_kind, ViolationKind::Cpu);
    }
}
