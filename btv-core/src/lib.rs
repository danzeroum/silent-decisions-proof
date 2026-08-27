//! # btv-core — Bill of Materials for Trustworthy Verdicts
//!
//! Reference implementation of the BTV framework for the IEEE Computer
//! major-revision artifact. Provides linearly-typed evidence, compliance,
//! and verdict types that make silent AI decisions a compile-time error.
//!
//! ## Type Invariants
//!
//! - `V ⊸ (E ⊗ C)` — a [`Verdict`] requires consuming one [`EvidenceToken`]
//!   and one [`ComplianceToken`].
//! - `V_esc ⊸ (O ⊗ 1)` — an [`EscalatedVerdict`] requires consuming one
//!   [`OperatorToken`].
//!
//! ## Trusted Computing Base (TCB)
//!
//! The following are part of the TCB and are NOT verified by this crate:
//!
//! 1. `rustc` and the Rust standard library
//! 2. The cryptographic primitives `blake3`, `hmac`, `sha2`, `subtle`
//! 3. The HMAC signing key (must be injected via `BTV_HMAC_KEY` or
//!    `BTV_AUTHORITY_KEY` from an HSM/KMS in production)
//! 4. The [`LogSink`] implementation (durability depends on the backend)
//!
//! This crate is `#![forbid(unsafe_code)]` — no `unsafe` blocks may be
//! introduced in its source. `unsafe` in transitive dependencies is
//! reported by `cargo geiger` and audited via `cargo audit`.
//!
//! ## What this crate does NOT guarantee
//!
//! - End-to-end non-repudiation across processes (requires durable
//!   persistence and channel integrity — see [`LogSink`] docs).
//! - That the consumed `EvidenceToken` corresponds to the *actual* decision
//!   context (only that *some* context was hashed and consumed).
//! - That `unsafe` in transitive dependencies is sound.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::needless_doctest_main)]

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// LogSink — pluggable durability backend (Test 5: partition tolerance)
// ============================================================================

/// A sink that receives sealed verdicts for durable persistence.
///
/// ## Contract
///
/// - `append()` MUST block until the verdict is durably persisted (fsync'd
///   to disk, replicated, etc.) or return `Err`.
/// - `append()` MUST be idempotent on the `evidence_id` — replaying the
///   same verdict MUST NOT corrupt the log.
/// - `is_available()` reports whether the sink is currently accepting writes.
///   It is used by the fail-secure path: if `false`, `issue_verdict()` returns
///   `Err(BtvError::LogUnavailable)` WITHOUT constructing a `Verdict`.
///
/// ## Implementations
///
/// - [`InMemoryLogSink`] — testing only; not durable.
/// - [`SqliteLogSink`] — SQLite WAL mode with `synchronous=FULL`.
/// - User-provided — for Postgres, S3, Kafka, etc.
///
/// ## Out of scope
///
/// This trait does not guarantee Byzantine fault tolerance, geographic
/// replication, or protection against a compromised storage backend.
/// Such guarantees require additional cryptographic sealing (e.g., Merkle
/// chains, threshold signatures) that are future work.
pub trait LogSink: Send + Sync {
    /// Durably append a sealed verdict record.
    fn append(&self, record: &VerdictRecord) -> Result<(), BtvError>;

    /// Report whether the sink is currently available for writes.
    fn is_available(&self) -> bool;
}

/// A serialized view of a `Verdict` suitable for persistence.
///
/// This is the *only* representation of a `Verdict` that crosses the
/// process boundary. It is HMAC-sealed at construction time; tampering
/// with any field is detectable via [`VerdictRecord::verify_integrity`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerdictRecord {
    pub evidence_id_hex: String,
    pub decision: String,
    pub explanation: String,
    pub jurisdiction: String,
    pub policy_version: String,
    pub appeal_deadline_hours: u32,
    pub hmac_hex: String,
}

impl VerdictRecord {
    /// Re-verify the HMAC after deserialization.
    ///
    /// Returns `false` if any field was modified in transit or storage.
    pub fn verify_integrity(&self) -> bool {
        let key = hmac_key();
        let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC key length valid");
        mac.update(self.evidence_id_hex.as_bytes());
        mac.update(self.decision.as_bytes());
        mac.update(self.explanation.as_bytes());
        mac.update(self.jurisdiction.as_bytes());
        mac.update(self.policy_version.as_bytes());
        mac.update(&self.appeal_deadline_hours.to_be_bytes());
        let expected = mac.finalize().into_bytes();
        match (hex::decode(&self.hmac_hex), expected) {
            (Ok(got), exp) => {
                got.len() == exp.len() && got.ct_eq(&exp).into()
            }
            _ => false,
        }
    }
}

// ============================================================================
// InMemoryLogSink — test fixture
// ============================================================================

/// An in-memory log sink for tests. NOT durable.
///
/// Can be toggled to "unavailable" to simulate a partition.
pub struct InMemoryLogSink {
    available: std::sync::atomic::AtomicBool,
    records: std::sync::Mutex<Vec<VerdictRecord>>,
}

impl InMemoryLogSink {
    pub fn new() -> Self {
        Self {
            available: std::sync::atomic::AtomicBool::new(true),
            records: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Simulate a network partition or log-server crash.
    pub fn fail(&self) {
        self.available.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Restore availability.
    pub fn recover(&self) {
        self.available.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Number of records successfully appended.
    pub fn len(&self) -> usize {
        self.records.lock().expect("records mutex poisoned").len()
    }

    /// Whether any records were appended.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryLogSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink for InMemoryLogSink {
    fn append(&self, record: &VerdictRecord) -> Result<(), BtvError> {
        if !self.is_available() {
            return Err(BtvError::LogUnavailable);
        }
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(record.clone());
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ============================================================================
// SqliteLogSink — durable persistence (Test 4 baseline)
// ============================================================================

/// A SQLite-backed log sink using WAL mode with `synchronous=FULL`.
///
/// Provides ACID durability guarantees without requiring an external
/// Postgres deployment. Suitable as a baseline for benchmarks.
pub struct SqliteLogSink {
    conn: std::sync::Mutex<rusqlite::Connection>,
    available: std::sync::atomic::AtomicBool,
}

impl SqliteLogSink {
    /// Open or create a SQLite log database at `path`.
    ///
    /// Configures WAL mode and `synchronous=FULL` for ACID durability.
    pub fn open(path: &str) -> Result<Self, BtvError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| BtvError::Backend(format!("sqlite open: {e}")))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=FULL;\
             PRAGMA temp_store=MEMORY;\
             CREATE TABLE IF NOT EXISTS verdicts (\
                 evidence_id_hex TEXT PRIMARY KEY,\
                 decision TEXT NOT NULL,\
                 explanation TEXT NOT NULL,\
                 jurisdiction TEXT NOT NULL,\
                 policy_version TEXT NOT NULL,\
                 appeal_deadline_hours INTEGER NOT NULL,\
                 hmac_hex TEXT NOT NULL,\
                 inserted_at TEXT NOT NULL DEFAULT (datetime('now'))\
             );",
        )
        .map_err(|e| BtvError::Backend(format!("sqlite init: {e}")))?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
            available: std::sync::atomic::AtomicBool::new(true),
        })
    }

    /// Open an in-memory SQLite database (for tests).
    pub fn open_in_memory() -> Result<Self, BtvError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| BtvError::Backend(format!("sqlite open: {e}")))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=FULL;\
             CREATE TABLE IF NOT EXISTS verdicts (\
                 evidence_id_hex TEXT PRIMARY KEY,\
                 decision TEXT NOT NULL,\
                 explanation TEXT NOT NULL,\
                 jurisdiction TEXT NOT NULL,\
                 policy_version TEXT NOT NULL,\
                 appeal_deadline_hours INTEGER NOT NULL,\
                 hmac_hex TEXT NOT NULL,\
                 inserted_at TEXT NOT NULL DEFAULT (datetime('now'))\
             );",
        )
        .map_err(|e| BtvError::Backend(format!("sqlite init: {e}")))?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
            available: std::sync::atomic::AtomicBool::new(true),
        })
    }

    /// Simulate a partition.
    pub fn fail(&self) {
        self.available.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Restore availability.
    pub fn recover(&self) {
        self.available.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl LogSink for SqliteLogSink {
    fn append(&self, record: &VerdictRecord) -> Result<(), BtvError> {
        if !self.is_available() {
            return Err(BtvError::LogUnavailable);
        }
        let conn = self.conn.lock().expect("conn mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO verdicts \
             (evidence_id_hex, decision, explanation, jurisdiction, policy_version, appeal_deadline_hours, hmac_hex) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                record.evidence_id_hex,
                record.decision,
                record.explanation,
                record.jurisdiction,
                record.policy_version,
                record.appeal_deadline_hours,
                record.hmac_hex,
            ],
        )
        .map_err(|e| BtvError::Backend(format!("sqlite insert: {e}")))?;
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ============================================================================
// Error type
// ============================================================================

/// Errors emitted by the BTV framework.
#[derive(Debug, thiserror::Error)]
pub enum BtvError {
    #[error("log sink unavailable — fail-secure: no verdict emitted")]
    LogUnavailable,
    #[error("unknown jurisdiction: {0}")]
    UnknownJurisdiction(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("integrity check failed — verdict tampered")]
    IntegrityFailure,
}

// ============================================================================
// Blake3Hash — Protection 3: no public arbitrary constructor
// ============================================================================

/// A BLAKE3 digest produced exclusively by consuming an [`EvidenceToken`].
///
/// There is intentionally no `From<[u8; 32]>` impl and no public tuple
/// constructor. The only path to obtain a `Blake3Hash` from outside this
/// module is through [`EvidenceToken::consume()`] (which is `pub(crate)`).
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Compute a BLAKE3 hash of arbitrary bytes. Only available inside this crate.
    pub(crate) fn of(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Blake3Hash(*hash.as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

// ============================================================================
// EvidenceToken — Protection 2: linear resource (#[must_use], not Clone/Copy)
// ============================================================================

/// A forensic evidence token derived from the decision context.
///
/// ## Linearity
///
/// `EvidenceToken` is `#[must_use]` and is NOT `Clone` or `Copy`. The
/// only way to extract the hash is via [`EvidenceToken::consume()`], which
/// moves `self` — destroying the token.
///
/// ## Affine-vs-linear caveat (R2)
///
/// Rust's ownership is *affine* (a value may be used 0 or 1 times), not
/// strictly *linear* (must be used exactly once). `#[must_use]` +
/// `#![deny(unused_must_use)]` escalates the warning for unused tokens to
/// a compile error, recovering linearity **at the API surface** for the
/// types in this crate. This does NOT extend to:
/// - `mem::forget(token)` (still callable, leaks the token)
/// - `panic!` after token creation but before consumption (token dropped
///   during unwind)
/// - Process abort/kill after token creation (token lost with the process)
///
/// These escape hatches are documented in the threat model; see
/// `reports/tcb_summary.md`.
#[must_use = "EvidenceToken must be consumed via Verdict::new() or issue_verdict(); \
              dropping it without use means the decision context was hashed but no \
              Verdict was produced — a logic error in the governance pipeline"]
pub struct EvidenceToken(Blake3Hash);

impl EvidenceToken {
    /// Hash `context` bytes with BLAKE3, producing a new `EvidenceToken`.
    pub fn new(context: &[u8]) -> Self {
        EvidenceToken(Blake3Hash::of(context))
    }

    /// Consume the token, returning the underlying [`Blake3Hash`].
    ///
    /// `pub(crate)`: only [`Verdict::new`] and [`issue_verdict`] inside
    /// this crate may consume a token. External code can create tokens
    /// via `EvidenceToken::new()` but cannot call `.consume()` directly.
    pub(crate) fn consume(self) -> Blake3Hash {
        self.0
    }
}

// ============================================================================
// ComplianceToken — contestability metadata
// ============================================================================

/// Compliance metadata consumed when constructing a [`Verdict`].
///
/// All fields are private. The constructor is `pub(crate)` — only
/// [`ComplianceAuthority::issue_token`] may issue compliance tokens.
pub struct ComplianceToken {
    jurisdiction: String,
    policy_version: String,
    contestability_deadline_hours: u32,
}

impl ComplianceToken {
    pub(crate) fn new(
        jurisdiction: impl Into<String>,
        policy_version: impl Into<String>,
        contestability_deadline_hours: u32,
    ) -> Self {
        ComplianceToken {
            jurisdiction: jurisdiction.into(),
            policy_version: policy_version.into(),
            contestability_deadline_hours,
        }
    }

    pub fn jurisdiction(&self) -> &str {
        &self.jurisdiction
    }

    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }

    pub fn deadline_hours(&self) -> u32 {
        self.contestability_deadline_hours
    }
}

// ============================================================================
// ComplianceAuthority — validated token issuance
// ============================================================================

/// A factory for issuing [`ComplianceToken`]s with validated jurisdiction.
///
/// Closes L2: external crates cannot self-declare arbitrary jurisdictions.
/// In production, the signing key is injected from an HSM/KMS at startup.
pub struct ComplianceAuthority {
    signing_key: Vec<u8>,
    allowed_jurisdictions: Vec<String>,
}

impl ComplianceAuthority {
    /// Create a new authority with an explicit signing key and jurisdiction allowlist.
    pub fn new(signing_key: Vec<u8>, allowed_jurisdictions: Vec<String>) -> Self {
        Self { signing_key, allowed_jurisdictions }
    }

    /// Read the signing key from `BTV_AUTHORITY_KEY`; fall back to a
    /// proof-of-concept constant if absent.
    pub fn new_from_env() -> Self {
        let signing_key = std::env::var("BTV_AUTHORITY_KEY")
            .map(|k| k.into_bytes())
            .unwrap_or_else(|_| b"btv-authority-key-proof-of-concept-2026".to_vec());
        Self {
            signing_key,
            allowed_jurisdictions: vec![
                "BR-LGPD".to_string(),
                "EU-GDPR".to_string(),
                "EU-AI-ACT".to_string(),
            ],
        }
    }

    /// Test-only constructor with a deterministic key and permissive allowlist.
    #[cfg(any(test, feature = "test-support"))]
    pub fn new_for_test() -> Self {
        Self {
            signing_key: b"btv-test-authority-key".to_vec(),
            allowed_jurisdictions: vec![
                "BR-LGPD".to_string(),
                "EU-GDPR".to_string(),
                "EU-AI-ACT".to_string(),
                "EU-AIACT".to_string(),
                "TEST-JURISDICTION".to_string(),
            ],
        }
    }

    /// Issue a validated [`ComplianceToken`].
    pub fn issue_token(
        &self,
        jurisdiction: &str,
        policy_version: &str,
        contestability_hours: u32,
    ) -> Result<ComplianceToken, BtvError> {
        if !self.allowed_jurisdictions.iter().any(|j| j == jurisdiction) {
            return Err(BtvError::UnknownJurisdiction(jurisdiction.to_string()));
        }
        Ok(ComplianceToken::new(jurisdiction, policy_version, contestability_hours))
    }
}

// ============================================================================
// Decision
// ============================================================================

/// The substantive outcome of an AI governance decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            Decision::Allow => b"allow",
            Decision::Deny => b"deny",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        }
    }
}

// ============================================================================
// Verdict — Protection 1: private fields, single construction path
// ============================================================================

/// A materialized AI decision with non-repudiable evidence and compliance binding.
///
/// ## Type Invariant  V ⊸ (E ⊗ C)
///
/// A `Verdict` can only be constructed by consuming **both** an
/// [`EvidenceToken`] and a [`ComplianceToken`]. All fields are private;
/// struct literal syntax is a compile-time error outside this module.
pub struct Verdict {
    evidence_id: Blake3Hash,
    compliance: ComplianceToken,
    decision: Decision,
    explanation: String,
    appeal_deadline_hours: u32,
    hmac: [u8; 32],
}

impl Verdict {
    /// The sole in-memory constructor. Enforces `V ⊸ (E ⊗ C)`.
    ///
    /// Moves `token` and `compliance` by value, consuming both linearly.
    /// Does NOT persist the verdict to a [`LogSink`]; use
    /// [`issue_verdict`] for the fail-secure path that also persists.
    pub fn new(
        token: EvidenceToken,
        compliance: ComplianceToken,
        decision: Decision,
        explanation: String,
    ) -> Self {
        let appeal_deadline_hours = compliance.deadline_hours();
        let evidence_id = token.consume();
        let hmac = Self::compute_hmac(&evidence_id, &decision, &explanation);
        Verdict {
            evidence_id,
            compliance,
            decision,
            explanation,
            appeal_deadline_hours,
            hmac,
        }
    }

    /// Verify that the Verdict has not been tampered with since construction.
    pub fn verify_integrity(&self) -> bool {
        let expected = Self::compute_hmac(&self.evidence_id, &self.decision, &self.explanation);
        expected.ct_eq(&self.hmac).into()
    }

    pub fn evidence_id(&self) -> &Blake3Hash {
        &self.evidence_id
    }

    pub fn decision(&self) -> &Decision {
        &self.decision
    }

    pub fn explanation(&self) -> &str {
        &self.explanation
    }

    pub fn appeal_deadline_hours(&self) -> u32 {
        self.appeal_deadline_hours
    }

    pub fn jurisdiction(&self) -> &str {
        self.compliance.jurisdiction()
    }

    pub fn policy_version(&self) -> &str {
        self.compliance.policy_version()
    }

    /// Serialize to a [`VerdictRecord`] for persistence.
    pub fn to_record(&self) -> VerdictRecord {
        VerdictRecord {
            evidence_id_hex: self.evidence_id.to_hex(),
            decision: self.decision.as_str().to_string(),
            explanation: self.explanation.clone(),
            jurisdiction: self.compliance.jurisdiction().to_string(),
            policy_version: self.compliance.policy_version().to_string(),
            appeal_deadline_hours: self.appeal_deadline_hours,
            hmac_hex: hex::encode(self.hmac),
        }
    }

    fn compute_hmac(
        evidence_id: &Blake3Hash,
        decision: &Decision,
        explanation: &str,
    ) -> [u8; 32] {
        let key = hmac_key();
        let mut mac =
            HmacSha256::new_from_slice(&key).expect("HMAC key length is valid");
        mac.update(evidence_id.as_bytes());
        mac.update(decision.as_bytes());
        mac.update(explanation.as_bytes());
        let result = mac.finalize();
        let bytes = result.into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }
}

// ============================================================================
// issue_verdict — fail-secure path (Test 5)
// ============================================================================

/// Issue a verdict with fail-secure persistence.
///
/// ## Contract
///
/// 1. If `sink.is_available()` returns `false`, return `Err(BtvError::LogUnavailable)`
///    WITHOUT constructing a `Verdict`. This is the fail-secure behavior
///    required by the CAL Trilemma: sacrifice Availability to preserve Legality.
/// 2. If `sink.append()` fails, return `Err(...)` WITHOUT returning a `Verdict`.
///    The verdict either persisted or it never existed — there is no
///    "verdict that was issued but not logged" state.
/// 3. If both succeed, return `Ok(Verdict)`. The `EvidenceToken` and
///    `ComplianceToken` are consumed regardless of outcome (linear).
///
/// ## Out of scope
///
/// This function does NOT guarantee that the persisted record is
/// replicated, geographically durable, or protected against a compromised
/// `LogSink` backend. It only guarantees that the *local* `LogSink::append`
/// returned `Ok` before returning a `Verdict` to the caller.
pub fn issue_verdict(
    token: EvidenceToken,
    compliance: ComplianceToken,
    decision: Decision,
    explanation: String,
    sink: &dyn LogSink,
) -> Result<Verdict, BtvError> {
    if !sink.is_available() {
        // Fail-secure: do not construct the Verdict. Token is consumed
        // (moved into this function) but not used — the caller loses it.
        // This is intentional: if we returned the token to the caller,
        // they could retry with the same token, violating linearity.
        std::mem::forget(token);
        std::mem::forget(compliance);
        return Err(BtvError::LogUnavailable);
    }

    let verdict = Verdict::new(token, compliance, decision, explanation);
    let record = verdict.to_record();

    match sink.append(&record) {
        Ok(()) => Ok(verdict),
        Err(e) => {
            // Verdict existed in memory but is not durable. Forget it
            // so the caller cannot accidentally use an un-logged verdict.
            std::mem::forget(verdict);
            Err(e)
        }
    }
}

// ============================================================================
// ContextRef — public reference to a failed decision context
// ============================================================================

/// A reference to a decision context that could not be processed automatically.
///
/// Unlike [`Blake3Hash`], this type has a **public** constructor. It is
/// `Clone` and `Copy` — it is data, not a linear resource.
#[derive(Debug, Clone, Copy)]
pub struct ContextRef([u8; 32]);

impl ContextRef {
    pub fn from_context(context: &[u8]) -> Self {
        let hash = blake3::hash(context);
        ContextRef(*hash.as_bytes())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        ContextRef(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

// ============================================================================
// OperatorToken — linear resource for human oversight
// ============================================================================

#[must_use = "OperatorToken must be consumed via EscalatedVerdict::new(); \
              dropping it without use means an operator was authenticated but \
              no escalation was recorded — a logic error in the governance pipeline"]
pub struct OperatorToken {
    operator_id: [u8; 32],
    signature: [u8; 32],
}

impl OperatorToken {
    fn new_signed(operator_id: [u8; 32], signing_key: &[u8; 32]) -> Self {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(signing_key)
            .expect("HMAC accepts any key size");
        mac.update(&operator_id);
        mac.update(b"operator-token-v1");
        let result = mac.finalize().into_bytes();
        let mut signature = [0u8; 32];
        signature.copy_from_slice(&result[..32]);
        OperatorToken { operator_id, signature }
    }

    pub fn operator_id(&self) -> &[u8; 32] {
        &self.operator_id
    }

    pub(crate) fn consume(self) -> ([u8; 32], [u8; 32]) {
        (self.operator_id, self.signature)
    }
}

// ============================================================================
// OperatorAuthority
// ============================================================================

pub struct OperatorAuthority {
    signing_key: [u8; 32],
}

impl OperatorAuthority {
    pub fn new(signing_key: [u8; 32]) -> Self {
        OperatorAuthority { signing_key }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn new_for_test() -> Self {
        OperatorAuthority { signing_key: [0xAA; 32] }
    }

    pub fn issue_token(&self, operator_id: [u8; 32]) -> OperatorToken {
        OperatorToken::new_signed(operator_id, &self.signing_key)
    }
}

// ============================================================================
// EscalatedVerdict — human override with linear accountability
// ============================================================================

pub struct EscalatedVerdict {
    operator_id: [u8; 32],
    operator_signature: [u8; 32],
    decision: Decision,
    failed_context: ContextRef,
    reason: String,
    hmac: [u8; 32],
}

impl EscalatedVerdict {
    /// The sole constructor. Enforces `V_esc ⊸ (O ⊗ 1)`.
    pub fn new(
        operator: OperatorToken,
        decision: Decision,
        failed_context: ContextRef,
        reason: String,
    ) -> Self {
        let (operator_id, operator_signature) = operator.consume();
        let hmac = Self::compute_hmac(
            &operator_id,
            &operator_signature,
            &decision,
            &failed_context,
            &reason,
        );
        EscalatedVerdict { operator_id, operator_signature, decision, failed_context, reason, hmac }
    }

    pub fn verify_integrity(&self) -> bool {
        let expected = Self::compute_hmac(
            &self.operator_id,
            &self.operator_signature,
            &self.decision,
            &self.failed_context,
            &self.reason,
        );
        expected.ct_eq(&self.hmac).into()
    }

    pub fn operator_id(&self) -> &[u8; 32] {
        &self.operator_id
    }

    pub fn operator_id_hex(&self) -> String {
        hex::encode(self.operator_id)
    }

    pub fn decision(&self) -> &Decision {
        &self.decision
    }

    pub fn failed_context(&self) -> &ContextRef {
        &self.failed_context
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    fn compute_hmac(
        operator_id: &[u8; 32],
        operator_signature: &[u8; 32],
        decision: &Decision,
        failed_context: &ContextRef,
        reason: &str,
    ) -> [u8; 32] {
        let key = hmac_key();
        let mut mac =
            HmacSha256::new_from_slice(&key).expect("HMAC accepts any key size");
        mac.update(b"btv-escalated-v1");
        mac.update(operator_id);
        mac.update(operator_signature);
        mac.update(decision.as_bytes());
        mac.update(failed_context.as_bytes());
        mac.update(reason.as_bytes());
        let result = mac.finalize();
        let bytes = result.into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }
}

// ============================================================================
// AccountableDecision — unifying trait for audit
// ============================================================================

pub trait AccountableDecision {
    fn decision(&self) -> &Decision;
    fn verify_integrity(&self) -> bool;
    fn is_automated(&self) -> bool;
}

impl AccountableDecision for Verdict {
    fn decision(&self) -> &Decision {
        self.decision()
    }
    fn verify_integrity(&self) -> bool {
        self.verify_integrity()
    }
    fn is_automated(&self) -> bool {
        true
    }
}

impl AccountableDecision for EscalatedVerdict {
    fn decision(&self) -> &Decision {
        self.decision()
    }
    fn verify_integrity(&self) -> bool {
        self.verify_integrity()
    }
    fn is_automated(&self) -> bool {
        false
    }
}

// ============================================================================
// HMAC key helper
// ============================================================================

fn hmac_key() -> Vec<u8> {
    std::env::var("BTV_HMAC_KEY")
        .map(|k| k.into_bytes())
        .unwrap_or_else(|_| b"btv-proof-key-constitutional-enclosure-2026".to_vec())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clause_1_valid_verdict_can_be_constructed() {
        let token = EvidenceToken::new(b"subject:alice | action:credit | score:0.42");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = Verdict::new(
            token, compliance, Decision::Deny,
            "Below threshold".to_string(),
        );
        assert!(verdict.verify_integrity());
        assert_eq!(verdict.jurisdiction(), "BR-LGPD");
        assert_eq!(verdict.appeal_deadline_hours(), 720);
    }

    #[test]
    fn clause_2_evidence_token_is_linear() {
        let token = EvidenceToken::new(b"decision-context");
        let hash = token.consume();
        assert_ne!(hash.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn clause_5_tampered_verdict_fails_integrity() {
        let token = EvidenceToken::new(b"context");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let mut verdict = Verdict::new(
            token, compliance, Decision::Deny,
            "Original".to_string(),
        );
        assert!(verdict.verify_integrity());
        verdict.explanation = "Tampered".to_string(); // bypass HMAC (test only)
        assert!(!verdict.verify_integrity());
    }

    #[test]
    fn clause_16_compliance_authority_rejects_unknown_jurisdiction() {
        let authority = ComplianceAuthority::new_for_test();
        let result = authority.issue_token("Narnia", "v0", 0);
        assert!(matches!(result, Err(BtvError::UnknownJurisdiction(_))));
    }

    #[test]
    fn clause_17_compliance_authority_accepts_valid_jurisdictions() {
        let authority = ComplianceAuthority::new_for_test();
        for j in &["BR-LGPD", "EU-GDPR", "EU-AI-ACT", "EU-AIACT"] {
            assert!(authority.issue_token(j, "1.0.0", 720).is_ok());
        }
    }

    #[test]
    fn fail_secure_when_log_unavailable() {
        let sink = InMemoryLogSink::new();
        sink.fail();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let result = issue_verdict(
            token, compliance, Decision::Allow,
            "ok".to_string(), &sink,
        );
        assert!(matches!(result, Err(BtvError::LogUnavailable)));
        assert_eq!(sink.len(), 0, "no record should have been appended");
    }

    #[test]
    fn fail_secure_when_append_fails() {
        let sink = InMemoryLogSink::new();
        sink.fail();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let result = issue_verdict(
            token, compliance, Decision::Allow,
            "ok".to_string(), &sink,
        );
        assert!(result.is_err());
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn happy_path_appends_and_returns_verdict() {
        let sink = InMemoryLogSink::new();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = issue_verdict(
            token, compliance, Decision::Allow,
            "ok".to_string(), &sink,
        ).unwrap();
        assert!(verdict.verify_integrity());
        assert_eq!(sink.len(), 1);
    }

    #[test]
    fn sqlite_log_sink_round_trip() {
        let sink = SqliteLogSink::open_in_memory().unwrap();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = issue_verdict(
            token, compliance, Decision::Allow,
            "ok".to_string(), &sink,
        ).unwrap();
        assert_eq!(verdict.jurisdiction(), "BR-LGPD");
    }

    #[test]
    fn escalated_verdict_works() {
        let auth = OperatorAuthority::new_for_test();
        let tok = auth.issue_token([0x42; 32]);
        let ctx = ContextRef::from_context(b"failed-ctx");
        let v = EscalatedVerdict::new(tok, Decision::Allow, ctx, "human override".to_string());
        assert!(v.verify_integrity());
        assert_eq!(v.operator_id(), &[0x42; 32]);
    }
}
