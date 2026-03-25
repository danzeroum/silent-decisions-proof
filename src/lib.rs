//! # Silent Decisions Are Type Errors — Proof Library
//!
//! Machine-checkable proof for the Constitutional Enclosure Theorem.
//!
//! ## Core Law
//!
//! ```text
//! V ⊸ (E ⊗ C)
//! ```
//!
//! A Verdict (`V`) can only be produced by linearly consuming:
//! - `E` — an [`EvidenceToken`] (BLAKE3 hash of the decision context)
//! - `C` — a [`ComplianceToken`] (contestability deadline, jurisdiction, policy version)
//!
//! ## Three Protections
//!
//! 1. **Private struct fields** — `Verdict { ... }` literal syntax is blocked outside this module
//! 2. **Linear ownership** — [`EvidenceToken`] is not `Clone`, not `Copy`, marked `#[must_use]`
//! 3. **No public constructors** for [`Blake3Hash`]

use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

// ─────────────────────────────────────────────────────────────────────────────
// Blake3Hash — Protection 3: no public arbitrary constructor
// ─────────────────────────────────────────────────────────────────────────────

/// A BLAKE3 digest produced exclusively by consuming an [`EvidenceToken`].
///
/// There is intentionally no `From<[u8; 32]>` impl and no public tuple constructor.
/// The only path to obtain a `Blake3Hash` from outside this module is through
/// [`EvidenceToken::consume()`].
pub struct Blake3Hash([u8; 32]);
// ↑ The inner field is private (default visibility).
//   Blake3Hash([0u8; 32]) from outside this module → compile error E0451.

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

// ─────────────────────────────────────────────────────────────────────────────
// EvidenceToken — Protection 2: linear resource (#[must_use], not Clone/Copy)
// ─────────────────────────────────────────────────────────────────────────────

/// A forensic evidence token derived from the decision context.
///
/// ## Linearity Guarantee
///
/// `EvidenceToken` is:
/// - `#[must_use]` — the compiler warns (deny → errors) when dropped without use
/// - Not `Clone` — cannot be duplicated
/// - Not `Copy` — move semantics enforced by the compiler
///
/// The only way to extract the hash is via [`EvidenceToken::consume()`], which
/// MOVES `self`, destroying the token. Rust's ownership system makes it a
/// compile-time error to use the token after it has been consumed.
#[must_use = "EvidenceToken must be consumed via .consume() to construct a Verdict; \
              dropping it without use means the decision context was hashed but no \
              Verdict was produced — a logic error in the governance pipeline"]
pub struct EvidenceToken(Blake3Hash);
// NO #[derive(Clone, Copy)] — linearity is enforced structurally.

impl EvidenceToken {
    /// Hash `context` bytes with BLAKE3, producing a new `EvidenceToken`.
    pub fn new(context: &[u8]) -> Self {
        EvidenceToken(Blake3Hash::of(context))
    }

    /// Consume the token, returning the underlying [`Blake3Hash`].
    ///
    /// Takes `self` **by value** — the token is destroyed after this call.
    /// The Rust compiler rejects any subsequent use of the moved token.
    ///
    /// Visibility is `pub(crate)`: only `Verdict::new()` inside this crate
    /// may consume a token. External code can create tokens via `EvidenceToken::new()`
    /// but cannot call `.consume()` directly — the only way to satisfy `#[must_use]`
    /// from outside the crate is to pass the token into `Verdict::new()`.
    pub(crate) fn consume(self) -> Blake3Hash {
        self.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ComplianceToken — contestability metadata
// ─────────────────────────────────────────────────────────────────────────────

/// Compliance metadata consumed when constructing a [`Verdict`].
///
/// Encodes regulatory obligations: jurisdiction, policy version, and the
/// deadline by which an affected party may contest the decision.
///
/// All fields are **private**. The constructor is `pub(crate)`, signalling
/// that only the governance kernel (Policy Engine) may issue compliance tokens.
/// External code receives compliance metadata by reading a [`Verdict`]'s
/// accessors — it cannot fabricate a `ComplianceToken` from scratch.
pub struct ComplianceToken {
    jurisdiction: String,
    policy_version: String,
    contestability_deadline_hours: u32,
}

impl ComplianceToken {
    /// Create a `ComplianceToken`.
    ///
    /// **Proof-repo note:** This constructor is `pub` so that the binary demo
    /// (a separate Rust crate in this package) can construct tokens directly.
    /// In the production BTV kernel (`BuildToValueGovernance`) this function is
    /// `pub(crate)` — only the Policy Engine, internal to the governance kernel,
    /// may issue compliance tokens. External callers receive compliance metadata
    /// by reading a [`Verdict`]'s accessors, never by constructing tokens directly.
    ///
    /// The proof's structural guarantee — that no silent decision can compile —
    /// is provided by the private *fields*, not by this constructor's visibility.
    /// Struct literal syntax (`ComplianceToken { jurisdiction: ... }`) is blocked
    /// from outside this module regardless of whether `new()` is `pub` or `pub(crate)`.
    pub fn new(
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

// ─────────────────────────────────────────────────────────────────────────────
// Decision
// ─────────────────────────────────────────────────────────────────────────────

/// The substantive outcome of an AI governance decision.
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
}

// ─────────────────────────────────────────────────────────────────────────────
// Verdict — Protection 1: private fields, single construction path
// ─────────────────────────────────────────────────────────────────────────────

/// A materialized AI decision with non-repudiable evidence and compliance binding.
///
/// ## Type Invariant  V ⊸ (E ⊗ C)
///
/// A `Verdict` can only be constructed by consuming **both**:
/// - an [`EvidenceToken`] (providing causal, non-repudiable evidence)
/// - a [`ComplianceToken`] (providing contestability rights)
///
/// All fields are **private**. Struct literal syntax (`Verdict { ... }`) is a
/// compile-time error outside this module. The sole constructor is
/// [`Verdict::new()`].
pub struct Verdict {
    evidence_id: Blake3Hash,          // private — only settable via Verdict::new()
    compliance: ComplianceToken,      // private — only settable via Verdict::new()
    decision: Decision,
    explanation: String,
    appeal_deadline_hours: u32,
    hmac: [u8; 32],                   // integrity seal over evidence + decision + explanation
}

impl Verdict {
    /// The sole constructor. Enforces `V ⊸ (E ⊗ C)`.
    ///
    /// Moves `token` and `compliance` by value, consuming both linearly.
    /// After this call, neither token nor compliance exist anywhere else.
    pub fn new(
        token: EvidenceToken,
        compliance: ComplianceToken,
        decision: Decision,
        explanation: String,
    ) -> Self {
        let appeal_deadline_hours = compliance.deadline_hours();
        let evidence_id = token.consume(); // ← linear consumption: token is destroyed here
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
    ///
    /// Returns `false` if any of `evidence_id`, `decision`, or `explanation`
    /// have been modified after the HMAC was sealed.
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

    fn compute_hmac(
        evidence_id: &Blake3Hash,
        decision: &Decision,
        explanation: &str,
    ) -> [u8; 32] {
        // LIMITATION (declared): The HMAC key is static in this proof repository.
        // This suffices to demonstrate integrity detection (clause_5) but does not
        // provide the key-management guarantees of production deployment.
        // Production BTV derives the signing key from an HSM; see Section 6 of the paper.
        let mut mac =
            HmacSha256::new_from_slice(b"btv-proof-key-constitutional-enclosure-2026")
                .expect("HMAC key length is valid");
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

// ─────────────────────────────────────────────────────────────────────────────
// Test-only tampering surface (not available in production builds)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
impl Verdict {
    /// Mutate the explanation WITHOUT updating the HMAC.
    /// Used exclusively in clause_5 to demonstrate that `verify_integrity()`
    /// detects post-construction tampering.
    pub(crate) fn tamper_explanation_for_test(&mut self, new_explanation: &str) {
        self.explanation = new_explanation.to_string();
        // HMAC is intentionally NOT updated — this simulates an adversarial mutation.
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EXTENSION: EscalatedVerdict — Human Override with Linear Accountability
// ═══════════════════════════════════════════════════════════════════════════
//
// Corollary 4.8: V_esc ⊸ (O ⊗ 1)
//
// An EscalatedVerdict can only be constructed by consuming exactly one
// OperatorToken. This is a SEPARATE type from Verdict — the Constitutional
// Enclosure Theorem (Theorem 4.6) remains unchanged.
//
// Design rationale: when EvidenceToken cannot be produced (system failure,
// timeout, unavailable model), the only well-typed alternative is human
// escalation — not a silent decision, not an unevidenced automated verdict.

// ─────────────────────────────────────────────────────────────────────────
// ContextRef — public reference to a failed decision context
// ─────────────────────────────────────────────────────────────────────────

/// A reference to the decision context that could not be processed automatically.
///
/// Unlike [`Blake3Hash`], this type has a **public** constructor.
/// It exists specifically to allow external code to reference a context
/// without exposing the private internals of `Blake3Hash`.
///
/// `ContextRef` is `Clone` and `Copy` — it is data, not a linear resource.
#[derive(Debug, Clone, Copy)]
pub struct ContextRef([u8; 32]);

impl ContextRef {
    /// Create a `ContextRef` from a BLAKE3 hash of the original context bytes.
    pub fn from_context(context: &[u8]) -> Self {
        let hash = blake3::hash(context);
        ContextRef(*hash.as_bytes())
    }

    /// Create a `ContextRef` from raw bytes (e.g., from a stored hash).
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

// ─────────────────────────────────────────────────────────────────────────
// OperatorToken — linear resource for human oversight
// ─────────────────────────────────────────────────────────────────────────

/// A cryptographic attestation that a specific human operator has assumed
/// accountability for a decision.
///
/// ## Linearity Guarantee
///
/// `OperatorToken` follows the same discipline as [`EvidenceToken`]:
/// - `#[must_use]` — compiler warns/errors when dropped unused
/// - Not `Clone` — cannot be duplicated
/// - Not `Copy` — move semantics enforced
///
/// The only way to consume an `OperatorToken` is through
/// [`EscalatedVerdict::new()`].
#[must_use = "OperatorToken must be consumed via EscalatedVerdict::new(); \
              dropping it without use means an operator was authenticated but \
              no escalation was recorded — a logic error in the governance pipeline"]
pub struct OperatorToken {
    operator_id: [u8; 32],
    signature: [u8; 32],
}

// NO #[derive(Clone, Copy)] — linearity enforced structurally.

impl OperatorToken {
    /// Private constructor — only callable by `OperatorAuthority::issue_token()`.
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

    /// Read-only access to the operator's identity.
    pub fn operator_id(&self) -> &[u8; 32] {
        &self.operator_id
    }

    /// Consume the token, returning the operator ID and signature.
    ///
    /// `pub(crate)`: only `EscalatedVerdict::new()` may consume.
    pub(crate) fn consume(self) -> ([u8; 32], [u8; 32]) {
        (self.operator_id, self.signature)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// OperatorAuthority — factory for OperatorToken issuance
// ─────────────────────────────────────────────────────────────────────────

/// An authority that issues [`OperatorToken`]s.
///
/// In production, the signing key would be injected from an HSM/KMS.
pub struct OperatorAuthority {
    signing_key: [u8; 32],
}

impl OperatorAuthority {
    /// Create an authority with a given signing key.
    pub fn new(signing_key: [u8; 32]) -> Self {
        OperatorAuthority { signing_key }
    }

    /// Test-only constructor with a deterministic key.
    #[cfg(any(test, feature = "test-support"))]
    pub fn new_for_test() -> Self {
        OperatorAuthority { signing_key: [0xAA; 32] }
    }

    /// Issue an `OperatorToken` for a given operator identity.
    pub fn issue_token(&self, operator_id: [u8; 32]) -> OperatorToken {
        OperatorToken::new_signed(operator_id, &self.signing_key)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// EscalatedVerdict — human override with linear accountability
// ─────────────────────────────────────────────────────────────────────────

/// A human-escalated decision with non-repudiable operator binding.
///
/// ## Type Invariant  V_esc ⊸ (O ⊗ 1)
///
/// An `EscalatedVerdict` can only be constructed by consuming exactly one
/// [`OperatorToken`]. The sole constructor is [`EscalatedVerdict::new()`].
///
/// All fields are **private**. Struct literal syntax is a compile-time error
/// outside this module.
pub struct EscalatedVerdict {
    operator_id: [u8; 32],
    operator_signature: [u8; 32],
    decision: Decision,
    failed_context: ContextRef,
    reason: String,
    hmac: [u8; 32],
}

impl EscalatedVerdict {
    /// The sole constructor. Enforces V_esc ⊸ (O ⊗ 1).
    ///
    /// Moves `operator` by value, consuming it linearly.
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

    /// Verify that the EscalatedVerdict has not been tampered with.
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
        let mut mac =
            HmacSha256::new_from_slice(b"btv-escalated-proof-key-2026-xx")
                .expect("HMAC accepts any key size");
        mac.update(b"btv-escalated-v1"); // canonical schema version prefix
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

    /// Test-only: tamper with reason to verify HMAC detection.
    #[cfg(test)]
    pub fn tamper_reason_for_test(&mut self, new_reason: &str) {
        self.reason = new_reason.to_string();
    }
}

// ─────────────────────────────────────────────────────────────────────────
// AccountableDecision — unifying trait for audit
// ─────────────────────────────────────────────────────────────────────────

/// A trait implemented by all BTV decision types.
///
/// Allows auditing code to process both `Verdict` and `EscalatedVerdict`
/// uniformly without knowing which variant was produced.
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

// ─────────────────────────────────────────────────────────────────────────
// Proof — 6 clauses of the Constitutional Enclosure Theorem
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod proof {
    use super::*;

    /// Clause 1: The happy path — a valid Verdict can be constructed when both
    /// EvidenceToken and ComplianceToken are provided.
    ///
    /// Establishes that the type system does not over-constrain: the correct
    /// usage pattern compiles and runs without error.
    #[test]
    fn clause_1_valid_verdict_can_be_constructed() {
        let token = EvidenceToken::new(
            b"subject:alice | action:credit-application | score:0.42 | threshold:0.50",
        );
        let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720); // 30 days per LGPD Art. 18§2
        let verdict = Verdict::new(
            token,
            compliance,
            Decision::Deny,
            "Credit score 0.42 is below the required threshold of 0.50.".to_string(),
        );

        assert!(
            verdict.verify_integrity(),
            "Freshly constructed Verdict must pass integrity check"
        );
        assert_eq!(verdict.jurisdiction(), "BR-LGPD");
        assert_eq!(verdict.appeal_deadline_hours(), 720);
    }

    /// Clause 2: EvidenceToken is a linear resource.
    ///
    /// Once consumed, the token no longer exists. The Rust ownership system
    /// enforces this at compile time: any attempt to use `token` after
    /// `token.consume()` produces error E0382 ("use of moved value").
    ///
    /// The compile-time guarantee cannot be demonstrated in a passing test by
    /// definition — a test that exercises the forbidden path would not compile.
    /// Clause 3 of the trybuild suite (below) demonstrates the negative.
    /// This clause demonstrates the positive: consume() works exactly once.
    #[test]
    fn clause_2_evidence_token_is_linear() {
        let token = EvidenceToken::new(b"decision-context-for-linearity-proof");
        let hash = token.consume(); // token is moved and destroyed here

        // The following line would produce E0382 at compile time:
        // let _second = token.consume(); // error[E0382]: use of moved value: `token`

        // Verify the hash is non-trivial (not all zeros)
        assert_ne!(
            hash.as_bytes(),
            &[0u8; 32],
            "Blake3 hash of non-empty input must not be the zero array"
        );

        // Verify the Blake3Hash reflects the input (deterministic)
        let token2 = EvidenceToken::new(b"decision-context-for-linearity-proof");
        let hash2 = token2.consume();
        assert_eq!(
            hash.as_bytes(),
            hash2.as_bytes(),
            "Same context must produce same evidence hash (determinism)"
        );

        // Verify different inputs produce different hashes (collision resistance)
        let token3 = EvidenceToken::new(b"different-decision-context");
        let hash3 = token3.consume();
        assert_ne!(
            hash.as_bytes(),
            hash3.as_bytes(),
            "Different contexts must produce different evidence hashes"
        );
    }

    /// Clause 3: Verdict struct literal syntax is blocked outside the defining module.
    ///
    /// `Verdict { evidence_id: ..., ... }` cannot be written outside `lib.rs`
    /// because all fields are private. The compiler rejects it with E0451.
    ///
    /// Verified via trybuild: `tests/ui/verdict_struct_literal.rs` must fail to compile.
    #[test]
    fn clause_3_verdict_struct_literal_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/verdict_struct_literal.rs");
    }

    /// Clause 4: Blake3Hash has no public arbitrary constructor.
    ///
    /// External code cannot forge a `Blake3Hash` by writing `Blake3Hash([0u8; 32])`
    /// because the inner field is private. Construction is only possible through
    /// `EvidenceToken::consume()`, which requires a genuine `EvidenceToken`.
    ///
    /// Verified via trybuild: `tests/ui/blake3hash_public_constructor.rs` must fail.
    #[test]
    fn clause_4_blake3hash_has_no_public_constructor() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/blake3hash_public_constructor.rs");
    }

    /// Clause 5: A tampered Verdict fails the integrity check.
    ///
    /// The HMAC seals evidence_id + decision + explanation at construction time.
    /// Any post-construction mutation of these fields is detectable via
    /// `verify_integrity()`. This clause uses a test-only tampering method
    /// (unavailable in production builds) to demonstrate the detection.
    #[test]
    fn clause_5_tampered_verdict_fails_integrity_check() {
        let token = EvidenceToken::new(b"credit-denial-production-context-2026");
        let compliance = ComplianceToken::new("EU-AIACT", "2024/1689", 720);
        let mut verdict = Verdict::new(
            token,
            compliance,
            Decision::Deny,
            "Risk score 0.87 exceeds the 0.75 threshold under EU AI Act Art. 86.".to_string(),
        );

        assert!(
            verdict.verify_integrity(),
            "Pre-tamper: Verdict must pass integrity check"
        );

        // Simulate adversarial post-construction mutation
        verdict.tamper_explanation_for_test("Approved — risk score within limits.");

        assert!(
            !verdict.verify_integrity(),
            "Post-tamper: Verdict must FAIL integrity check after explanation mutation"
        );
    }

    /// Clause 6: Dropping an EvidenceToken without consuming it produces a compiler warning.
    ///
    /// The `#[must_use]` attribute on `EvidenceToken` causes the compiler to
    /// emit a warning when a token is created and then dropped without calling
    /// `.consume()`. With `#[deny(unused_must_use)]`, this escalates to a
    /// compile error — verified in `tests/ui/dropped_evidence_token.rs`.
    ///
    /// This test verifies the positive: proper consumption silences the warning.
    #[test]
    fn clause_6_dropped_token_produces_compiler_warning() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/dropped_evidence_token.rs");
    }

    /// Clause 7: External code cannot call EvidenceToken::consume() directly.
    ///
    /// `consume()` is `pub(crate)`. External callers can create `EvidenceToken`
    /// values via `EvidenceToken::new()` but cannot invoke `.consume()` —
    /// the only way to satisfy `#[must_use]` from outside the crate is to pass
    /// the token into `Verdict::new()`.
    ///
    /// Verified via trybuild: `tests/ui/external_consume_call.rs` must fail to compile.
    #[test]
    fn clause_7_external_consume_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/external_consume_call.rs");
    }

    // ═══════════════════════════════════════════════════════════════
    // COROLLARY 4.8 — EscalatedVerdict proof clauses
    // ═══════════════════════════════════════════════════════════════

    /// Clause 8: EscalatedVerdict can be constructed with valid OperatorToken.
    #[test]
    fn clause_8_escalated_verdict_can_be_constructed() {
        let authority = OperatorAuthority::new_for_test();
        let token = authority.issue_token([0x42; 32]);
        let ctx = ContextRef::from_context(b"medical-triage-context-timeout");
        let verdict = EscalatedVerdict::new(
            token,
            Decision::Allow,
            ctx,
            "System timeout during triage — nurse approved immediate treatment".to_string(),
        );
        assert!(
            verdict.verify_integrity(),
            "Freshly constructed EscalatedVerdict must pass integrity check"
        );
        assert_eq!(verdict.operator_id(), &[0x42; 32]);
        assert_eq!(verdict.reason(), "System timeout during triage — nurse approved immediate treatment");
    }

    /// Clause 9: OperatorToken is a linear resource (consumed on use).
    #[test]
    fn clause_9_operator_token_is_linear() {
        let authority = OperatorAuthority::new_for_test();
        let token = authority.issue_token([0x01; 32]);
        let (id, sig) = token.consume(); // token is moved and destroyed
        // let _second = token.consume(); // would produce E0382
        assert_eq!(id, [0x01; 32]);
        assert_ne!(sig, [0u8; 32], "Signature must be non-trivial");
    }

    /// Clause 10: EscalatedVerdict struct literal is blocked.
    #[test]
    fn clause_10_escalated_struct_literal_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/escalated_struct_literal.rs");
    }

    /// Clause 11: OperatorToken cannot be reused (linear consumption).
    #[test]
    fn clause_11_operator_token_reuse_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/escalated_token_reuse.rs");
    }

    /// Clause 12: Dropping OperatorToken without use produces compile error.
    #[test]
    fn clause_12_dropped_operator_token_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/escalated_operator_token_drop.rs");
    }

    /// Clause 13: External code cannot call OperatorToken::consume().
    #[test]
    fn clause_13_external_consume_is_blocked() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/ui/escalated_consume_external.rs");
    }

    /// Clause 14: Tampered EscalatedVerdict fails integrity check.
    #[test]
    fn clause_14_tampered_escalated_verdict_fails_integrity() {
        let authority = OperatorAuthority::new_for_test();
        let token = authority.issue_token([0x42; 32]);
        let ctx = ContextRef::from_context(b"triage-context");
        let mut verdict = EscalatedVerdict::new(
            token,
            Decision::Allow,
            ctx,
            "Legitimate escalation reason".to_string(),
        );
        assert!(verdict.verify_integrity(), "Pre-tamper: must pass");
        verdict.tamper_reason_for_test("Maliciously altered reason");
        assert!(!verdict.verify_integrity(), "Post-tamper: must FAIL");
    }

    /// Clause 15: AccountableDecision trait works for both types.
    #[test]
    fn clause_15_accountable_decision_trait_is_polymorphic() {
        let token = EvidenceToken::new(b"auto-context");
        let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720);
        let auto_verdict = Verdict::new(
            token, compliance, Decision::Deny,
            "Auto denial".to_string(),
        );

        let authority = OperatorAuthority::new_for_test();
        let op_token = authority.issue_token([0x42; 32]);
        let ctx = ContextRef::from_context(b"failed-context");
        let esc_verdict = EscalatedVerdict::new(
            op_token, Decision::Allow, ctx,
            "Human override".to_string(),
        );

        fn check(d: &dyn AccountableDecision) -> bool {
            d.verify_integrity()
        }

        assert!(check(&auto_verdict));
        assert!(check(&esc_verdict));
        assert!(auto_verdict.is_automated());
        assert!(!esc_verdict.is_automated());
    }
}
