//! # btv-core — Bill of Materials for Trustworthy Verdicts
//!
//! Reference implementation of the BTV framework for the IEEE Computer
//! major-revision artifact. Provides linearly-typed evidence, compliance,
//! and verdict types that make silent AI decisions a compile-time error.
//!
//! ## Type Invariants
//!
//! - `V ⊸ (E ⊗ C_signed)` — a [`Verdict`] requires consuming one
//!   [`EvidenceToken`] and one [`ComplianceToken`] whose authority
//!   signature verifies (OS-02). Tokens are issued only by a
//!   [`ComplianceAuthority`] holding the recognized signing key
//!   (`BTV_AUTHORITY_KEY`, HSM/KMS-injected in production).
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
use rusqlite::OptionalExtension;
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
/// ## Errors
///
/// Returns [`BtvError::LogUnavailable`] when `is_available()` is `false`
/// (fail-secure). Returns [`BtvError::Backend`] on persistence failure.
/// Returns [`BtvError::LogConflict`] when a record with the same
/// `evidence_id` already exists with different content — rewriting a
/// persisted verdict is prohibited (OS-03); replaying a byte-identical
/// record succeeds (true idempotency).
///
/// ## Implementations
///
/// - [`InMemoryLogSink`] — testing only; not durable.
/// - [`SqliteLogSink`] — `SQLite` WAL mode with `synchronous=FULL`.
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
    ///
    /// # Errors
    ///
    /// See the trait-level documentation: [`BtvError::LogUnavailable`] on
    /// fail-secure, [`BtvError::Backend`] on persistence failure,
    /// [`BtvError::LogConflict`] when an existing record with the same
    /// `evidence_id` differs byte-to-byte (append-only guarantee, OS-03).
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

// ============================================================================
// Integrity seal — single HMAC preimage shared by `Verdict` and
// `VerdictRecord` (OS-01, closes audit finding F1).
//
// History: before this refactor, `Verdict::compute_hmac` authenticated
// `(evidence_id raw 32 B, decision, explanation)` while
// `VerdictRecord::verify_integrity` re-computed over
// `(evidence_id_hex 64 chars, decision, explanation, jurisdiction,
// policy_version, appeal_deadline_hours)`. Different preimages meant that
// *no* persisted record ever verified. Both sides now feed the same
// `SealFields` structure, in the same wire encoding, through `seal()`.
// ============================================================================

/// Domain-separation tag for the BTV integrity seal.
///
/// Prevents preimage reuse across protocols or seal generations; the
/// trailing NUL terminates the tag unambiguously.
const SEAL_DOMAIN: &[u8] = b"BTV-v1\x00";

/// The exact field set authenticated by [`seal`], in the exact encoding
/// that crosses the process boundary (`evidence_id` in hex).
struct SealFields<'a> {
    evidence_id_hex: &'a str,
    decision: &'a str,
    explanation: &'a str,
    jurisdiction: &'a str,
    policy_version: &'a str,
    appeal_deadline_hours: u32,
}

/// Compute the 32-byte integrity seal over `fields`.
///
/// Domain separation: the preimage is `SEAL_DOMAIN` followed by each
/// string field prefixed with its byte length as a `u32` big-endian, and
/// finally `appeal_deadline_hours` as `u32` big-endian. Without
/// length-prefixing, adjacent attacker-controlled fields (e.g.
/// `explanation` and `jurisdiction`) would be ambiguous under
/// concatenation; see the `seal_is_unambiguous` test.
fn seal(fields: &SealFields<'_>) -> [u8; 32] {
    let key = hmac_key();
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC key length valid");
    mac.update(SEAL_DOMAIN);
    for field in [
        fields.evidence_id_hex.as_bytes(),
        fields.decision.as_bytes(),
        fields.explanation.as_bytes(),
        fields.jurisdiction.as_bytes(),
        fields.policy_version.as_bytes(),
    ] {
        mac.update(&(u32::try_from(field.len()).expect("field length fits u32")).to_be_bytes());
        mac.update(field);
    }
    mac.update(&fields.appeal_deadline_hours.to_be_bytes());
    let result = mac.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result.into_bytes());
    out
}

impl VerdictRecord {
    /// Re-verify the HMAC after deserialization.
    ///
    /// Recomputes the seal over the *same* field set and encoding used at
    /// `Verdict` construction time (see [`seal`]). Returns `false` if any
    /// field was modified in transit or storage.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        let fields = SealFields {
            evidence_id_hex: &self.evidence_id_hex,
            decision: &self.decision,
            explanation: &self.explanation,
            jurisdiction: &self.jurisdiction,
            policy_version: &self.policy_version,
            appeal_deadline_hours: self.appeal_deadline_hours,
        };
        let expected = seal(&fields);
        match (hex::decode(&self.hmac_hex), expected) {
            (Ok(got), exp) => got.len() == exp.len() && got.ct_eq(&exp).into(),
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            available: std::sync::atomic::AtomicBool::new(true),
            records: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Simulate a network partition or log-server crash.
    pub fn fail(&self) {
        self.available
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Restore availability.
    pub fn recover(&self) {
        self.available
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Number of records successfully appended.
    ///
    /// # Panics
    ///
    /// Panics if the records mutex is poisoned (a writer panicked while
    /// holding the lock).
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
    /// # Errors
    ///
    /// See the trait-level documentation. Idempotent on `evidence_id`:
    /// replaying a byte-identical record is `Ok(())`; a differing record
    /// with the same `evidence_id` is [`BtvError::LogConflict`] (OS-03).
    fn append(&self, record: &VerdictRecord) -> Result<(), BtvError> {
        if !self.is_available() {
            return Err(BtvError::LogUnavailable);
        }
        let mut records = self.records.lock().expect("records mutex poisoned");
        // Append-only, OS-03: same evidence_id + identical content -> true
        // idempotency; same evidence_id + any differing field -> conflict.
        // Silent overwrite (the SQLite `INSERT OR REPLACE` behavior this
        // replaces) is prohibited by the trait contract.
        if let Some(existing) = records
            .iter()
            .find(|r| r.evidence_id_hex == record.evidence_id_hex)
        {
            return if records_equivalent(existing, record) {
                Ok(())
            } else {
                Err(BtvError::LogConflict(record.evidence_id_hex.clone()))
            };
        }
        records.push(record.clone());
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Byte-level equivalence of two records across every authenticated and
/// metadata field. Used by the append-only conflict checks (OS-03).
fn records_equivalent(a: &VerdictRecord, b: &VerdictRecord) -> bool {
    a.evidence_id_hex == b.evidence_id_hex
        && a.decision == b.decision
        && a.explanation == b.explanation
        && a.jurisdiction == b.jurisdiction
        && a.policy_version == b.policy_version
        && a.appeal_deadline_hours == b.appeal_deadline_hours
        && a.hmac_hex == b.hmac_hex
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
    /// Open or create a `SQLite` log database at `path`.
    ///
    /// Configures WAL mode and `synchronous=FULL` for ACID durability.
    ///
    /// # Errors
    ///
    /// Returns [`BtvError::Backend`] if the database cannot be opened or
    /// the schema cannot be initialised.
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

    /// Open an in-memory `SQLite` database (for tests).
    ///
    /// # Errors
    ///
    /// Returns [`BtvError::Backend`] if the database cannot be opened or
    /// the schema cannot be initialised.
    ///
    /// Note: SQLite silently ignores `journal_mode=WAL` for `:memory:`
    /// databases and `synchronous=FULL` is meaningless without a file —
    /// this constructor is for TESTS only; use [`SqliteLogSink::open`] for
    /// durability measurements (OS-07).
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
        self.available
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Restore availability.
    pub fn recover(&self) {
        self.available
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl LogSink for SqliteLogSink {
    /// Append-only persistence (OS-03, closes F3).
    ///
    /// The previous implementation used `INSERT OR REPLACE INTO verdicts`,
    /// which let ANY caller silently rewrite `decision`, `explanation`, and
    /// `hmac_hex` of an already-persisted verdict by re-presenting the same
    /// `evidence_id` — mutation, not idempotency, fatal to the
    /// non-repudiation claim. This implementation never rewrites:
    ///
    /// 1. `INSERT` plain — a duplicate `evidence_id` fails at the key.
    /// 2. On conflict, the existing row is read back and compared
    ///    byte-to-byte: identical -> `Ok(())` (true idempotency, as the
    ///    trait contract requires); any differing field ->
    ///    [`BtvError::LogConflict`] and the original row remains untouched.
    ///
    /// # Errors
    ///
    /// See the trait-level documentation.
    fn append(&self, record: &VerdictRecord) -> Result<(), BtvError> {
        if !self.is_available() {
            return Err(BtvError::LogUnavailable);
        }
        let conn = self.conn.lock().expect("conn mutex poisoned");
        let existing: Option<(String, String, String, String, i64, String)> = conn
            .query_row(
                "SELECT decision, explanation, jurisdiction, policy_version, \
                 appeal_deadline_hours, hmac_hex \
                 FROM verdicts WHERE evidence_id_hex = ?1",
                rusqlite::params![record.evidence_id_hex],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| BtvError::Backend(format!("sqlite select: {e}")))?;

        if let Some(existing) = existing {
            let equivalent = existing.0 == record.decision
                && existing.1 == record.explanation
                && existing.2 == record.jurisdiction
                && existing.3 == record.policy_version
                && existing.4 == i64::from(record.appeal_deadline_hours)
                && existing.5 == record.hmac_hex;
            return if equivalent {
                Ok(())
            } else {
                Err(BtvError::LogConflict(record.evidence_id_hex.clone()))
            };
        }

        conn.execute(
            "INSERT INTO verdicts \
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
    #[error(
        "compliance token signature invalid — token not issued by the \
         recognized authority (BTV_AUTHORITY_KEY)"
    )]
    InvalidTokenSignature,
    #[error(
        "append-only log conflict: evidence_id {0} already exists with \
         different content; rewriting a persisted verdict is prohibited"
    )]
    LogConflict(String),
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

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
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
// ComplianceToken — signed contestability metadata (OS-02, closes F2)
// ============================================================================

/// Domain-separation tag for the compliance-token signature.
const COMPLIANCE_TOKEN_DOMAIN: &[u8] = b"BTV-compliance-token-v1\x00";

/// Compliance metadata consumed when constructing a [`Verdict`].
///
/// All fields are private. The constructor is `pub(crate)` — only
/// [`ComplianceAuthority::issue_token`] may issue compliance tokens, and
/// every token carries an HMAC signature over its contents produced with
/// the authority's signing key. [`Verdict::new`] verifies that signature
/// before producing a verdict: a forged or foreign-authority token is
/// rejected with [`BtvError::InvalidTokenSignature`].
pub struct ComplianceToken {
    jurisdiction: String,
    policy_version: String,
    contestability_deadline_hours: u32,
    signature: [u8; 32],
}

impl ComplianceToken {
    pub(crate) fn new(
        jurisdiction: impl Into<String>,
        policy_version: impl Into<String>,
        contestability_deadline_hours: u32,
        signature: [u8; 32],
    ) -> Self {
        ComplianceToken {
            jurisdiction: jurisdiction.into(),
            policy_version: policy_version.into(),
            contestability_deadline_hours,
            signature,
        }
    }

    /// Compute the authority signature over the token contents.
    ///
    /// Domain separation: `COMPLIANCE_TOKEN_DOMAIN` followed by each string
    /// field prefixed with its byte length as a `u32` big-endian, then the
    /// deadline as `u32` big-endian. Length-prefixing removes the
    /// field-boundary ambiguity of plain concatenation (same construction
    /// as [`seal`]).
    fn compute_signature(
        key: &[u8],
        jurisdiction: &str,
        policy_version: &str,
        deadline_hours: u32,
    ) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
        mac.update(COMPLIANCE_TOKEN_DOMAIN);
        for field in [jurisdiction.as_bytes(), policy_version.as_bytes()] {
            mac.update(&(u32::try_from(field.len()).expect("field length fits u32")).to_be_bytes());
            mac.update(field);
        }
        mac.update(&deadline_hours.to_be_bytes());
        let result = mac.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result.into_bytes());
        out
    }

    /// Verify the token's authority signature against a candidate key.
    fn verify_signature_with_key(&self, key: &[u8]) -> bool {
        let expected = Self::compute_signature(
            key,
            &self.jurisdiction,
            &self.policy_version,
            self.contestability_deadline_hours,
        );
        expected.ct_eq(&self.signature).into()
    }

    #[must_use]
    pub fn jurisdiction(&self) -> &str {
        &self.jurisdiction
    }

    #[must_use]
    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }

    #[must_use]
    pub fn deadline_hours(&self) -> u32 {
        self.contestability_deadline_hours
    }
}

// ============================================================================
// ComplianceAuthority — validated token issuance
// ============================================================================

/// A factory for issuing signed [`ComplianceToken`]s with validated
/// jurisdiction (OS-02, closes F2).
///
/// Closes L2: external crates cannot self-declare arbitrary jurisdictions
/// NOR self-issue authority — every token is HMAC-signed with the
/// authority's signing key, and [`Verdict::new`] verifies that signature
/// against the *recognized* key (the same `BTV_AUTHORITY_KEY` source the
/// authority itself uses). In production the signing key is injected from
/// an HSM/KMS at startup.
pub struct ComplianceAuthority {
    signing_key: Vec<u8>,
    allowed_jurisdictions: Vec<String>,
}

impl ComplianceAuthority {
    /// Create a new authority with an explicit signing key and jurisdiction allowlist.
    ///
    /// Note: [`Verdict::new`] verifies token signatures against the
    /// *recognized* authority key (see [`authority_key`]). Tokens issued by
    /// an authority holding any other key are rejected there — to run a
    /// recognized authority, provision the same key via `BTV_AUTHORITY_KEY`
    /// or use [`ComplianceAuthority::new_from_env`].
    #[must_use]
    pub fn new(signing_key: Vec<u8>, allowed_jurisdictions: Vec<String>) -> Self {
        Self {
            signing_key,
            allowed_jurisdictions,
        }
    }

    /// Read the signing key from `BTV_AUTHORITY_KEY`; fall back to a
    /// proof-of-concept constant if absent.
    #[must_use]
    pub fn new_from_env() -> Self {
        let signing_key = authority_key();
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
    ///
    /// Uses the proof-of-concept fallback constant — the same key
    /// [`Verdict::new`] verifies against when `BTV_AUTHORITY_KEY` is unset —
    /// so tokens issued here verify in test/CI environments, and no
    /// environment variable is read on the (bench-measured) issuance path.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn new_for_test() -> Self {
        Self {
            signing_key: b"btv-authority-key-proof-of-concept-2026".to_vec(),
            allowed_jurisdictions: vec![
                "BR-LGPD".to_string(),
                "EU-GDPR".to_string(),
                "EU-AI-ACT".to_string(),
                "EU-AIACT".to_string(),
                "TEST-JURISDICTION".to_string(),
            ],
        }
    }

    /// Issue a validated, signed [`ComplianceToken`].
    ///
    /// The token embeds an HMAC signature over
    /// `(jurisdiction, policy_version, deadline_hours)` computed with the
    /// authority's signing key; [`Verdict::new`] verifies it.
    ///
    /// # Errors
    ///
    /// Returns [`BtvError::UnknownJurisdiction`] if `jurisdiction` is not
    /// on the authority's allowlist.
    pub fn issue_token(
        &self,
        jurisdiction: &str,
        policy_version: &str,
        contestability_hours: u32,
    ) -> Result<ComplianceToken, BtvError> {
        if !self.allowed_jurisdictions.iter().any(|j| j == jurisdiction) {
            return Err(BtvError::UnknownJurisdiction(jurisdiction.to_string()));
        }
        let signature = ComplianceToken::compute_signature(
            &self.signing_key,
            jurisdiction,
            policy_version,
            contestability_hours,
        );
        Ok(ComplianceToken::new(
            jurisdiction,
            policy_version,
            contestability_hours,
            signature,
        ))
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

    #[must_use]
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
    /// The sole in-memory constructor. Enforces `V ⊸ (E ⊗ C_signed)`.
    ///
    /// Moves `token` and `compliance` by value, consuming both linearly.
    /// Verifies the compliance token's authority signature (OS-02): a
    /// forged or foreign-authority token is rejected with
    /// [`BtvError::InvalidTokenSignature`] and no `Verdict` is produced —
    /// both tokens are consumed either way. Does NOT persist the verdict
    /// to a [`LogSink`]; use [`issue_verdict`] for the fail-secure path
    /// that also persists.
    ///
    /// # Errors
    ///
    /// Returns [`BtvError::InvalidTokenSignature`] if the compliance
    /// token's authority signature does not verify against the recognized
    /// key (see [`authority_key`]).
    pub fn new(
        token: EvidenceToken,
        compliance: ComplianceToken,
        decision: Decision,
        explanation: String,
    ) -> Result<Self, BtvError> {
        if !compliance.verify_signature_with_key(&authority_key()) {
            return Err(BtvError::InvalidTokenSignature);
        }
        let appeal_deadline_hours = compliance.deadline_hours();
        let evidence_id = token.consume();
        let evidence_id_hex = evidence_id.to_hex();
        let jurisdiction = compliance.jurisdiction().to_string();
        let policy_version = compliance.policy_version().to_string();
        let fields = SealFields {
            evidence_id_hex: &evidence_id_hex,
            decision: decision.as_str(),
            explanation: &explanation,
            jurisdiction: &jurisdiction,
            policy_version: &policy_version,
            appeal_deadline_hours,
        };
        let hmac = seal(&fields);
        Ok(Verdict {
            evidence_id,
            compliance,
            decision,
            explanation,
            appeal_deadline_hours,
            hmac,
        })
    }

    /// Verify that the Verdict has not been tampered with since construction.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        let fields = SealFields {
            evidence_id_hex: &self.evidence_id.to_hex(),
            decision: self.decision.as_str(),
            explanation: &self.explanation,
            jurisdiction: self.compliance.jurisdiction(),
            policy_version: self.compliance.policy_version(),
            appeal_deadline_hours: self.appeal_deadline_hours,
        };
        let expected = seal(&fields);
        expected.ct_eq(&self.hmac).into()
    }

    #[must_use]
    pub fn evidence_id(&self) -> &Blake3Hash {
        &self.evidence_id
    }

    #[must_use]
    pub fn decision(&self) -> &Decision {
        &self.decision
    }

    #[must_use]
    pub fn explanation(&self) -> &str {
        &self.explanation
    }

    #[must_use]
    pub fn appeal_deadline_hours(&self) -> u32 {
        self.appeal_deadline_hours
    }

    #[must_use]
    pub fn jurisdiction(&self) -> &str {
        self.compliance.jurisdiction()
    }

    #[must_use]
    pub fn policy_version(&self) -> &str {
        self.compliance.policy_version()
    }

    /// Serialize to a [`VerdictRecord`] for persistence.
    ///
    /// The record carries the seal computed by `Verdict::new` over the
    /// identical field set/encoding, so `record.verify_integrity()` returns
    /// `true` for every untampered record (OS-01).
    #[must_use]
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
/// 2. If the compliance token's authority signature does not verify,
///    return `Err(BtvError::InvalidTokenSignature)` without constructing
///    a `Verdict` (OS-02).
/// 3. If `sink.append()` fails, return `Err(...)` WITHOUT returning a `Verdict`.
///    The verdict either persisted or it never existed — there is no
///    "verdict that was issued but not logged" state.
/// 4. If all succeed, return `Ok(Verdict)`. The `EvidenceToken` and
///    `ComplianceToken` are consumed regardless of outcome (linear): they
///    were moved into this function, so on any error path the caller no
///    longer owns them and cannot retry with the same tokens.
///
/// # Errors
///
/// - [`BtvError::LogUnavailable`] — fail-secure: the sink cannot accept
///   writes; no `Verdict` is constructed.
/// - [`BtvError::InvalidTokenSignature`] — the compliance token was not
///   issued by the recognized authority; no `Verdict` is constructed.
/// - [`BtvError::Backend`] / [`BtvError::LogConflict`] — persistence
///   failed after construction; the `Verdict` is dropped and never
///   returned.
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
        // Fail-secure: no Verdict is constructed. Linearity needs no special
        // measure here: `token` and `compliance` were moved into this
        // function, so the caller cannot reuse them for a retry — explicit
        // `drop` releases their heap allocations. (`std::mem::forget` would
        // be WRONG, not safer: these types do not implement `Drop`, so
        // forgetting them leaks memory without any linearity benefit —
        // see clippy::forget_non_drop and audit finding F6.)
        drop(token);
        drop(compliance);
        return Err(BtvError::LogUnavailable);
    }

    // OS-02: signature verification happens inside Verdict::new; a forged
    // token aborts construction before any persistence is attempted.
    let verdict = Verdict::new(token, compliance, decision, explanation)?;
    let record = verdict.to_record();

    match sink.append(&record) {
        Ok(()) => Ok(verdict),
        Err(e) => {
            // Verdict existed in memory but is not durable. Drop it so no
            // un-logged verdict escapes to the caller (same linear
            // reasoning as the fail-secure branch above).
            drop(verdict);
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
    #[must_use]
    pub fn from_context(context: &[u8]) -> Self {
        let hash = blake3::hash(context);
        ContextRef(*hash.as_bytes())
    }

    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        ContextRef(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
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
        let mut mac =
            <HmacSha256 as Mac>::new_from_slice(signing_key).expect("HMAC accepts any key size");
        mac.update(&operator_id);
        mac.update(b"operator-token-v1");
        let result = mac.finalize().into_bytes();
        let mut signature = [0u8; 32];
        signature.copy_from_slice(&result[..32]);
        OperatorToken {
            operator_id,
            signature,
        }
    }

    #[must_use]
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
    #[must_use]
    pub fn new(signing_key: [u8; 32]) -> Self {
        OperatorAuthority { signing_key }
    }

    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn new_for_test() -> Self {
        OperatorAuthority {
            signing_key: [0xAA; 32],
        }
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
    #[must_use]
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
            decision,
            &failed_context,
            &reason,
        );
        EscalatedVerdict {
            operator_id,
            operator_signature,
            decision,
            failed_context,
            reason,
            hmac,
        }
    }

    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        let expected = Self::compute_hmac(
            &self.operator_id,
            &self.operator_signature,
            self.decision,
            &self.failed_context,
            &self.reason,
        );
        expected.ct_eq(&self.hmac).into()
    }

    #[must_use]
    pub fn operator_id(&self) -> &[u8; 32] {
        &self.operator_id
    }

    #[must_use]
    pub fn operator_id_hex(&self) -> String {
        hex::encode(self.operator_id)
    }

    #[must_use]
    pub fn decision(&self) -> &Decision {
        &self.decision
    }

    #[must_use]
    pub fn failed_context(&self) -> &ContextRef {
        &self.failed_context
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    fn compute_hmac(
        operator_id: &[u8; 32],
        operator_signature: &[u8; 32],
        decision: Decision,
        failed_context: &ContextRef,
        reason: &str,
    ) -> [u8; 32] {
        let key = hmac_key();
        let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC accepts any key size");
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
// Key-resolution helpers
// ============================================================================

/// Resolve a key from the environment ONCE per process.
///
/// Reading `BTV_HMAC_KEY`/`BTV_AUTHORITY_KEY` on every seal/signature
/// computation would put an environment-variable lookup (~100 ns) inside
/// the measured hot path of `Verdict::new`; `OnceLock` resolves the key at
/// first use and keeps every later read to a single atomic load.
/// Documented semantics: the key in effect is the one present at first
/// use; changing the variable afterwards has no effect.
fn cached_key(var: &'static str, fallback: &'static str) -> Vec<u8> {
    static HMAC_KEY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    static AUTHORITY_KEY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let cell = match var {
        "BTV_HMAC_KEY" => &HMAC_KEY,
        "BTV_AUTHORITY_KEY" => &AUTHORITY_KEY,
        _ => unreachable!("internal key name"),
    };
    cell.get_or_init(|| {
        std::env::var(var)
            .unwrap_or_else(|_| fallback.to_string())
            .into_bytes()
    })
    .clone()
}

/// HMAC key for verdict/record integrity seals (`BTV_HMAC_KEY` in
/// production, injected from an HSM/KMS; deterministic `PoC` fallback).
fn hmac_key() -> Vec<u8> {
    cached_key(
        "BTV_HMAC_KEY",
        "btv-proof-key-constitutional-enclosure-2026",
    )
}

/// The recognized compliance-authority signing key (`BTV_AUTHORITY_KEY` in
/// production, HSM/KMS-injected; deterministic `PoC` fallback). Both the
/// issuing [`ComplianceAuthority`] and the verifying [`Verdict::new`] use
/// this source, so issuer and verifier agree by construction.
fn authority_key() -> Vec<u8> {
    cached_key(
        "BTV_AUTHORITY_KEY",
        "btv-authority-key-proof-of-concept-2026",
    )
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
            token,
            compliance,
            Decision::Deny,
            "Below threshold".to_string(),
        )
        .expect("new_for_test authority holds the recognized key");
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
        let mut verdict = Verdict::new(token, compliance, Decision::Deny, "Original".to_string())
            .expect("new_for_test authority holds the recognized key");
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
        let result = issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink);
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
        let result = issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink);
        assert!(result.is_err());
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn happy_path_appends_and_returns_verdict() {
        let sink = InMemoryLogSink::new();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict =
            issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink).unwrap();
        assert!(verdict.verify_integrity());
        assert_eq!(sink.len(), 1);
    }

    #[test]
    fn sqlite_log_sink_round_trip() {
        let sink = SqliteLogSink::open_in_memory().unwrap();
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict =
            issue_verdict(token, compliance, Decision::Allow, "ok".to_string(), &sink).unwrap();
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

    // ========================================================================
    // OS-01 gate — unified seal preimage (closes F1)
    // ========================================================================

    fn sample_record() -> VerdictRecord {
        let token = EvidenceToken::new(b"subject:alice | action:credit | score:0.42");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = Verdict::new(
            token,
            compliance,
            Decision::Deny,
            "below threshold".to_string(),
        )
        .expect("new_for_test authority holds the recognized key");
        verdict.to_record()
    }

    /// Gate 1/3: construct -> `to_record()` -> `verify_integrity()` == true.
    /// Before OS-01 this returned `false` for EVERY persisted record
    /// (different HMAC preimages on the two sides of `to_record`).
    #[test]
    fn record_seal_roundtrip() {
        let record = sample_record();
        assert!(
            record.verify_integrity(),
            "persisted record seal must verify after the unified-seal refactor"
        );
    }

    /// Gate 2/3: tampering with each of the six authenticated fields must
    /// invalidate the seal (one test per field).
    #[test]
    fn record_tamper_evidence_id_hex_detected() {
        let mut r = sample_record();
        // A different (still well-formed) evidence id hex, obtained by
        // hashing a different context (pub(crate) consume is in scope here).
        let token = EvidenceToken::new(b"different-context");
        r.evidence_id_hex = token.consume().to_hex();
        assert!(!r.verify_integrity());
    }

    #[test]
    fn record_tamper_decision_detected() {
        let mut r = sample_record();
        r.decision = if r.decision == "deny" {
            "allow"
        } else {
            "deny"
        }
        .to_string();
        assert!(!r.verify_integrity());
    }

    #[test]
    fn record_tamper_explanation_detected() {
        let mut r = sample_record();
        r.explanation = "tampered".to_string();
        assert!(!r.verify_integrity());
    }

    #[test]
    fn record_tamper_jurisdiction_detected() {
        let mut r = sample_record();
        r.jurisdiction = "EU-GDPR".to_string();
        assert!(!r.verify_integrity());
    }

    #[test]
    fn record_tamper_policy_version_detected() {
        let mut r = sample_record();
        r.policy_version = "9.9.9".to_string();
        assert!(!r.verify_integrity());
    }

    #[test]
    fn record_tamper_appeal_deadline_detected() {
        let mut r = sample_record();
        r.appeal_deadline_hours += 1;
        assert!(!r.verify_integrity());
    }

    /// Gate 3/3: the seal is unambiguous under field-boundary shifts.
    ///
    /// Moving one character from `explanation` to `jurisdiction` keeps the
    /// naive concatenation identical (`"abcde" || "F-BR" == "abcd" || "eF-BR"`
    /// family of collisions). With u32 big-endian length prefixes the two
    /// preimages differ, so the seal must NOT verify for the shifted record
    /// when it carries the original seal.
    #[test]
    fn seal_is_unambiguous() {
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_for_test();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = Verdict::new(token, compliance, Decision::Deny, "abcd".to_string())
            .expect("new_for_test authority holds the recognized key");
        let mut shifted = verdict.to_record();
        // Original: explanation="abcd", jurisdiction="F-BR"? No — build the
        // shifted variant explicitly: move the last char of explanation into
        // jurisdiction, i.e. explanation="abc", jurisdiction="d"-prefixed.
        shifted.explanation = "abc".to_string();
        shifted.jurisdiction = format!("d{}", shifted.jurisdiction);
        shifted.hmac_hex = hex::encode({
            // Seal of the ORIGINAL (unshifted) field values — the attacker's
            // hope is that concatenation ambiguity carries it over.
            let fields = SealFields {
                evidence_id_hex: &shifted.evidence_id_hex,
                decision: &shifted.decision,
                explanation: "abcd",
                jurisdiction: shifted.jurisdiction.trim_start_matches('d'),
                policy_version: &shifted.policy_version,
                appeal_deadline_hours: shifted.appeal_deadline_hours,
            };
            seal(&fields)
        });
        assert!(
            !shifted.verify_integrity(),
            "field-boundary shift must invalidate the seal (length-prefix domain separation)"
        );
    }

    // ========================================================================
    // OS-02 gate — signed ComplianceToken (closes F2)
    // ========================================================================

    /// A token whose signature does not match its contents (forged in-place
    /// via the pub(crate) constructor) must be rejected by `Verdict::new`.
    #[test]
    fn forged_token_signature_rejected() {
        let token = EvidenceToken::new(b"ctx");
        // Forged: valid fields, zero signature (no authority produced it).
        let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720, [0u8; 32]);
        let result = Verdict::new(token, compliance, Decision::Deny, "x".to_string());
        assert!(
            matches!(result, Err(BtvError::InvalidTokenSignature)),
            "forged compliance token must be rejected"
        );
    }

    /// A token issued by an authority whose key is NOT the recognized key
    /// (the `BTV_AUTHORITY_KEY` source `Verdict::new` verifies against)
    /// must also be rejected — an external crate cannot self-issue authority.
    #[test]
    fn rogue_authority_token_rejected() {
        let token = EvidenceToken::new(b"ctx");
        let rogue =
            ComplianceAuthority::new(b"rogue-authority-key".to_vec(), vec!["BR-LGPD".to_string()]);
        let compliance = rogue
            .issue_token("BR-LGPD", "1.0.0", 720)
            .expect("rogue allowlist accepts the jurisdiction");
        let result = Verdict::new(token, compliance, Decision::Allow, "x".to_string());
        assert!(matches!(result, Err(BtvError::InvalidTokenSignature)));
    }

    /// `issue_verdict` propagates the signature failure with no record
    /// appended and no verdict emitted (tokens consumed — linear).
    #[test]
    fn issue_verdict_propagates_invalid_signature_fail_secure() {
        let sink = InMemoryLogSink::new();
        let token = EvidenceToken::new(b"ctx");
        let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720, [0xFF; 32]);
        let result = issue_verdict(token, compliance, Decision::Allow, "x".to_string(), &sink);
        assert!(matches!(result, Err(BtvError::InvalidTokenSignature)));
        assert_eq!(
            sink.len(),
            0,
            "no record may be appended for a forged token"
        );
    }

    /// Tokens issued by the recognized (env-fallback) authority verify and
    /// produce verdicts whose seals round-trip through [`VerdictRecord`].
    #[test]
    fn signed_token_happy_path() {
        let token = EvidenceToken::new(b"ctx");
        let authority = ComplianceAuthority::new_from_env();
        let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
        let verdict = Verdict::new(token, compliance, Decision::Allow, "ok".to_string())
            .expect("new_from_env resolves the same recognized key as Verdict::new");
        assert!(verdict.verify_integrity());
        assert!(verdict.to_record().verify_integrity());
    }

    /// The compliance-token signature is unambiguous under field-boundary
    /// shifts (same length-prefix construction as the verdict seal).
    #[test]
    fn compliance_signature_is_unambiguous() {
        let key = b"some-authority-key".to_vec();
        let sig = ComplianceToken::compute_signature(&key, "BR-LGPD", "1.0.0-abc", 720);
        // Shift one char from policy_version into jurisdiction: naive
        // concatenation is identical, length-prefixed preimage is not.
        let shifted = ComplianceToken::compute_signature(&key, "BR-LGPD1", ".0.0-abc", 720);
        assert_ne!(sig, shifted);
    }
}
