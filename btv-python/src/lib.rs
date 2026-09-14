//! # btv-python — PyO3 binding for btv-core
//!
//! Exposes a **minimal, fail-secure** surface to Python:
//!
//! - `issue_verdict(raw_context: bytes, policy: str, jurisdiction: str = "BR-LGPD")`
//!   returns a `SealedVerdict` (read-only).
//! - `SealedVerdict` exposes `hash_hex`, `decision`, `jurisdiction`,
//!   `appeal_deadline_hours`, `verify_integrity()`. It does NOT expose
//!   `consume()`, the underlying `EvidenceToken`, or any constructor that
//!   accepts a pre-computed hash.
//! - `BTVError` translates `btv_core::BtvError` variants to Python exceptions.
//!
//! ## Threat model (L4 — polyglot boundary)
//!
//! The Python orchestrator **cannot**:
//! 1. Construct a `Verdict` from a pre-computed hash (no such API exposed).
//! 2. Reuse an `EvidenceToken` (the token is consumed inside Rust before
//!    Python ever sees the result).
//! 3. Drop a token silently — `__enter__`/`__exit__` enforce deterministic
//!    teardown; if `__exit__` is called before the verdict is materialized,
//!    the in-flight `EvidenceToken` is dropped on the Rust side with
//!    `#[must_use]` enforced (compile-time, not runtime).
//!
//! The Python orchestrator **can**:
//! - Pass arbitrary `bytes` as `raw_context`. Rust hashes them with BLAKE3.
//! - Read the resulting `SealedVerdict`'s public accessors.
//! - Call `verify_integrity()` to detect tampering of the serialized form.
//!
//! ## Out of scope
//!
//! - End-to-end non-repudiation across process boundaries.
//! - Protection against a compromised Python interpreter (a malicious
//!   extension could call `btv_core` directly via FFI).
//! - Durability of the `LogSink` beyond what the configured backend provides.

#![forbid(unsafe_code)]

use btv_core::{
    issue_verdict as rs_issue_verdict, BtvError as RsBtvError, ComplianceAuthority,
    Decision, EvidenceToken, InMemoryLogSink, LogSink, SqliteLogSink,
};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::create_exception;
use std::sync::Arc;

// ============================================================================
// Python-facing exception type
// ============================================================================

create_exception!(btv_python, BTVError, PyValueError);

// ============================================================================
// Error conversion: btv_core::BtvError → PyErr (raised as BTVError in Python)
// ============================================================================

fn to_py_err(e: RsBtvError) -> PyErr {
    match e {
        RsBtvError::LogUnavailable => {
            BTVError::new_err("BTVError: log unavailable — fail-secure, no verdict emitted")
        }
        RsBtvError::UnknownJurisdiction(j) => {
            BTVError::new_err(format!("BTVError: unknown jurisdiction: {j}"))
        }
        RsBtvError::Backend(msg) => {
            BTVError::new_err(format!("BTVError: backend: {msg}"))
        }
        RsBtvError::IntegrityFailure => {
            BTVError::new_err("BTVError: integrity check failed — verdict tampered")
        }
    }
}

// ============================================================================
// Python-facing SealedVerdict (read-only view of a btv_core::Verdict)
// ============================================================================

/// A sealed, read-only verdict returned by `issue_verdict()`.
///
/// Python code **cannot**:
/// - Construct this object directly (no `__init__`).
/// - Modify any field.
/// - Access the underlying `EvidenceToken` (it was consumed inside Rust).
/// - Call `consume()` (not exposed).
///
/// Python code **can**:
/// - Read `hash_hex`, `decision`, `jurisdiction`, `policy_version`,
///   `appeal_deadline_hours`, `explanation`.
/// - Call `verify_integrity()` to re-check the HMAC.
/// - Use as a context manager (`with issue_verdict(...) as v: ...`) for
///   deterministic teardown.
#[pyclass(name = "SealedVerdict")]
pub struct SealedVerdict {
    inner: Option<btv_core::Verdict>,
}

#[pymethods]
impl SealedVerdict {
    /// BLAKE3 hash of the original decision context, hex-encoded.
    #[getter]
    pub fn hash_hex(&self) -> PyResult<String> {
        match &self.inner {
            Some(v) => Ok(v.evidence_id().to_hex()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Decision outcome: "allow" or "deny".
    #[getter]
    pub fn decision(&self) -> PyResult<String> {
        match &self.inner {
            Some(v) => Ok(v.decision().as_str().to_string()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Jurisdiction code (e.g., "BR-LGPD", "EU-GDPR", "EU-AI-ACT").
    #[getter]
    pub fn jurisdiction(&self) -> PyResult<String> {
        match &self.inner {
            Some(v) => Ok(v.jurisdiction().to_string()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Policy version string.
    #[getter]
    pub fn policy_version(&self) -> PyResult<String> {
        match &self.inner {
            Some(v) => Ok(v.policy_version().to_string()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Appeal deadline in hours.
    #[getter]
    pub fn appeal_deadline_hours(&self) -> PyResult<u32> {
        match &self.inner {
            Some(v) => Ok(v.appeal_deadline_hours()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Explanation string.
    #[getter]
    pub fn explanation(&self) -> PyResult<String> {
        match &self.inner {
            Some(v) => Ok(v.explanation().to_string()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Re-verify the HMAC. Returns True if the verdict is intact.
    pub fn verify_integrity(&self) -> PyResult<bool> {
        match &self.inner {
            Some(v) => Ok(v.verify_integrity()),
            None => Err(PyRuntimeError::new_err("verdict already consumed")),
        }
    }

    /// Context manager: enter returns self.
    pub fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager: exit drops the inner verdict deterministically.
    ///
    /// This guarantees that the Verdict is freed at a known point, NOT
    /// relying on Python's GC. After __exit__, all accessors raise
    /// `RuntimeError("verdict already consumed")`.
    pub fn __exit__(&mut self, _exc_type: &Bound<'_, PyAny>, _exc_value: &Bound<'_, PyAny>, _traceback: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.inner = None; // Drop the Verdict now.
        Ok(false) // Don't suppress exceptions.
    }
}

// ============================================================================
// LogSink configuration
// ============================================================================

/// Configuration for the log sink used by `issue_verdict()`.
///
/// Defaults to `in-memory` (testing only). For durable persistence,
/// construct via `LogConfig.sqlite(path)`.
#[pyclass(name = "LogConfig", subclass)]
pub struct LogConfig {
    backend: Arc<dyn LogSink>,
}

#[pymethods]
impl LogConfig {
    /// Create an in-memory log sink (NOT durable — testing only).
    #[staticmethod]
    pub fn in_memory() -> Self {
        LogConfig {
            backend: Arc::new(InMemoryLogSink::new()),
        }
    }

    /// Create a SQLite-backed log sink with WAL mode and synchronous=FULL.
    #[staticmethod]
    pub fn sqlite(path: &str) -> PyResult<Self> {
        let sink = SqliteLogSink::open(path).map_err(to_py_err)?;
        Ok(LogConfig {
            backend: Arc::new(sink),
        })
    }

    /// Simulate a network partition or log-server crash.
    pub fn fail(&self) -> PyResult<()> {
        // We need a mutable handle to the AtomicBool. Arc<dyn LogSink>
        // doesn't expose fail() directly — we'd need a downcast. For the
        // test, we use a dedicated TestingLogConfig that exposes fail().
        Err(PyRuntimeError::new_err(
            "LogConfig.fail() requires TestingLogConfig — use btv_python.TestingLogConfig",
        ))
    }
}

/// Testing log config that exposes `fail()` and `recover()`.
#[pyclass(name = "TestingLogConfig", extends = LogConfig)]
pub struct TestingLogConfig {
    sink: Arc<InMemoryLogSink>,
}

#[pymethods]
impl TestingLogConfig {
    /// Tell pytest not to collect this class as a test class.
    #[classattr]
    const __test__: bool = false;

    #[new]
    fn new() -> (Self, LogConfig) {
        let sink = Arc::new(InMemoryLogSink::new());
        let sink_clone = Arc::clone(&sink);
        // We can't easily put the InMemoryLogSink behind Arc<dyn LogSink>
        // AND keep a typed handle for fail() — but we can, since
        // InMemoryLogSink: Send + Sync.
        let backend: Arc<dyn LogSink> = sink_clone;
        (TestingLogConfig { sink }, LogConfig { backend })
    }

    fn fail(&self) {
        self.sink.fail();
    }

    fn recover(&self) {
        self.sink.recover();
    }

    fn len(&self) -> usize {
        self.sink.len()
    }

    fn is_empty(&self) -> bool {
        self.sink.is_empty()
    }
}

// ============================================================================
// Top-level functions
// ============================================================================

/// Issue a verdict with fail-secure persistence.
///
/// ## Parameters
///
/// - `raw_context: bytes` — the decision context to hash with BLAKE3.
///   Must be `bytes`, NOT `str` or `dict`. Python code CANNOT pass a
///   pre-computed hash; the hash is computed inside the Rust enclave.
/// - `decision: str` — "allow" or "deny".
/// - `jurisdiction: str` — one of "BR-LGPD", "EU-GDPR", "EU-AI-ACT".
/// - `policy_version: str` — opaque version string for the policy applied.
/// - `explanation: str` — human-readable explanation.
/// - `contestability_hours: int` — appeal deadline in hours.
/// - `log_config: LogConfig` — persistence backend. Required.
///
/// ## Returns
///
/// A `SealedVerdict` instance. Use as a context manager for deterministic
/// teardown: `with issue_verdict(...) as v: ...`
///
/// ## Raises
///
/// - `TypeError` if `raw_context` is not `bytes`.
/// - `ValueError` if `decision` is not "allow"/"deny" or jurisdiction is unknown.
/// - `btv_python.BTVError` if the log sink is unavailable or persistence fails.
#[pyfunction]
#[pyo3(signature = (
    raw_context, decision, jurisdiction, policy_version,
    explanation, contestability_hours, log_config,
))]
pub fn issue_verdict<'py>(
    py: Python<'py>,
    raw_context: &Bound<'py, PyAny>,
    decision: &str,
    jurisdiction: &str,
    policy_version: &str,
    explanation: &str,
    contestability_hours: u32,
    log_config: &Bound<'py, PyAny>,
) -> PyResult<Py<SealedVerdict>> {
    // THREAT MODEL — Class F (forged hash attempt):
    // Python code MUST pass `bytes` for raw_context. Any other type
    // (dict, str, list, None) is rejected with TypeError. There is NO
    // API to pass a pre-computed hash.
    let raw_bytes: Vec<u8> = if let Ok(b) = raw_context.downcast::<PyBytes>() {
        b.as_bytes().to_vec()
    } else {
        return Err(PyTypeError::new_err(
            "raw_context must be bytes — pre-computed hashes are not accepted",
        ));
    };

    let dec = match decision {
        "allow" => Decision::Allow,
        "deny" => Decision::Deny,
        other => {
            return Err(PyValueError::new_err(format!(
                "decision must be 'allow' or 'deny', got: {other}"
            )));
        }
    };

    // Extract the LogSink from the LogConfig (or TestingLogConfig subclass).
    let log_config_obj: PyRef<'_, LogConfig> = log_config
        .extract()
        .map_err(|_| PyTypeError::new_err("log_config must be LogConfig or TestingLogConfig"))?;
    let sink: &dyn LogSink = log_config_obj.backend.as_ref();

    // Build the authority. In production, this would come from env / HSM.
    // Attribute-based cfg, NOT `if cfg!(...)`: both arms of an `if cfg!()`
    // are type-checked, and `new_for_test` only exists when btv-core's
    // `test-support` feature is enabled (it is `#[cfg(any(test, feature =
    // "test-support"))]` there). Building btv-python without the feature
    // must still compile — `cfg!()` masked this by never being exercised
    // in CI, which always passes `--features test-support`.
    #[cfg(feature = "test-support")]
    let authority = ComplianceAuthority::new_for_test();
    #[cfg(not(feature = "test-support"))]
    let authority = ComplianceAuthority::new_from_env();
    let compliance = authority
        .issue_token(jurisdiction, policy_version, contestability_hours)
        .map_err(to_py_err)?;

    // Construct the EvidenceToken INSIDE Rust. Python never sees it.
    let token = EvidenceToken::new(&raw_bytes);

    // Hold the GIL only during construction; the actual work is CPU-bound
    // and does not touch Python objects.
    let result = py.allow_threads(|| {
        rs_issue_verdict(token, compliance, dec, explanation.to_string(), sink)
    });

    match result {
        Ok(verdict) => Ok(Py::new(py, SealedVerdict { inner: Some(verdict) })?),
        Err(e) => Err(to_py_err(e)),
    }
}

// ============================================================================
// Module definition
// ============================================================================

#[pymodule]
fn btv_python(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SealedVerdict>()?;
    m.add("BTVError", py.get_type_bound::<BTVError>())?;
    m.add_class::<LogConfig>()?;
    m.add_class::<TestingLogConfig>()?;
    m.add_function(wrap_pyfunction!(issue_verdict, m)?)?;

    // Expose the module's __version__.
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Threat model docstring at module level.
    m.add(
        "__doc__",
        "btv-python — PyO3 binding for btv-core.\n\
         \n\
         Threat model (L4 — polyglot boundary):\n\
         - Python MUST pass `bytes` as raw_context. No API to pass a pre-computed hash.\n\
         - EvidenceToken is consumed inside Rust; Python never sees it.\n\
         - SealedVerdict is read-only; use `with` for deterministic teardown.\n\
         \n\
         Out of scope:\n\
         - End-to-end non-repudiation across processes.\n\
         - Protection against a compromised Python interpreter.\n\
         - Durability of the LogSink beyond what the backend provides.",
    )?;

    Ok(())
}
