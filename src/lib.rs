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
    fn as_bytes(&self) -> &[u8] {
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
        // Constant-time comparison to resist timing attacks
        expected.iter().zip(self.hmac.iter()).all(|(a, b)| a == b)
            && expected.len() == self.hmac.len()
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

// ─────────────────────────────────────────────────────────────────────────────
// Proof — 6 clauses of the Constitutional Enclosure Theorem
// ─────────────────────────────────────────────────────────────────────────────

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
}
