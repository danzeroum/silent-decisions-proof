//! # Silent Decisions Are Type Errors — Live Demo
//!
//! Demonstrates the Constitutional Enclosure Theorem in action:
//! every materialized Verdict consumed exactly one EvidenceToken
//! and one ComplianceToken. No silent decisions are possible.

use silent_decisions_proof::{ComplianceToken, Decision, EvidenceToken, Verdict};

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Silent Decisions Are Type Errors — Constitutional Enclosure");
    println!("  Proof Repository · IEEE Computer 2026");
    println!("═══════════════════════════════════════════════════════════════\n");

    // ── Scenario: Credit application denial ──────────────────────────────────
    println!("Scenario: AI credit-scoring system denies application\n");

    // Step 1: Hash the decision context into an EvidenceToken
    let context = b"subject:alice-silva | \
                    action:credit-application | \
                    model:credit-scorer-v3.1 | \
                    score:0.42 | \
                    threshold:0.50 | \
                    timestamp:2026-03-24T14:32:00Z";

    let token = EvidenceToken::new(context);
    println!("  [1] EvidenceToken created");
    println!("      Context hash: (token not yet consumed)");

    // Step 2: Prepare compliance metadata
    let compliance = ComplianceToken {
        jurisdiction: "BR-LGPD".to_string(),
        policy_version: "1.0.0".to_string(),
        contestability_deadline_hours: 720, // 30 days per LGPD Art. 18§2
    };
    println!("  [2] ComplianceToken prepared");
    println!("      Jurisdiction: BR-LGPD");
    println!("      Policy:       v1.0.0");
    println!("      Appeal window: 720 hours (30 days)\n");

    // Step 3: Construct the Verdict — this CONSUMES both tokens
    //
    //   V ⊸ (E ⊗ C)
    //
    // After this line, `token` and `compliance` no longer exist.
    // The Rust compiler enforces this — there is no way to "forget" the evidence.
    let verdict = Verdict::new(
        token,       // ← EvidenceToken consumed here (moved, destroyed)
        compliance,  // ← ComplianceToken consumed here (moved, destroyed)
        Decision::Deny,
        "Credit score 0.42 is below the required threshold of 0.50. \
         You may contest this decision within 720 hours."
            .to_string(),
    );

    println!("  [3] Verdict constructed — V ⊸ (E ⊗ C) satisfied");
    println!("      Decision:    {:?}", match verdict.decision() {
        Decision::Allow => "Allow",
        Decision::Deny  => "Deny",
    });
    println!("      Evidence ID: {}", verdict.evidence_id().to_hex());
    println!("      Explanation: {}", verdict.explanation());
    println!("      Appeal:      {} hours", verdict.appeal_deadline_hours());
    println!("      Jurisdiction: {}\n", verdict.jurisdiction());

    // Step 4: Verify integrity
    println!("  [4] Integrity check:");
    if verdict.verify_integrity() {
        println!("      ✓ PASS — HMAC seal verified\n");
    } else {
        println!("      ✗ FAIL — Verdict has been tampered with\n");
    }

    // ── What the type system prevents ───────────────────────────────────────
    println!("─────────────────────────────────────────────────────────────────");
    println!("  What the type system prevents (would not compile):");
    println!();
    println!("  // Error E0451 — private fields:");
    println!("  let v = Verdict {{ evidence_id: ..., ... }};");
    println!();
    println!("  // Error E0451 — private inner field:");
    println!("  let h = Blake3Hash([0u8; 32]);");
    println!();
    println!("  // Error E0382 — use of moved value:");
    println!("  let t = EvidenceToken::new(ctx);");
    println!("  let _h1 = t.consume();");
    println!("  let _h2 = t.consume(); // ← compile error");
    println!();
    println!("  // Warning (deny → error) — unused must_use:");
    println!("  EvidenceToken::new(ctx); // dropped without .consume()");
    println!("─────────────────────────────────────────────────────────────────");
    println!();
    println!("Run `cargo test` to verify all 6 proof clauses.");
}
