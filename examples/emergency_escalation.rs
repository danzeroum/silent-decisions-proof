//! Worked Example 4: Emergency escalation when evidence cannot be produced
//!
//! Simulates a scenario where the AI triage system fails (database timeout)
//! and the decision must be escalated to a human operator.
//! Demonstrates: EscalatedVerdict, OperatorToken linearity, AI Act Art. 14.

use silent_decisions_proof::{
    AccountableDecision, ContextRef, Decision, EscalatedVerdict, OperatorAuthority,
};

fn main() {
    println!("=== Worked Example 4: Emergency Human Escalation ===\n");

    // Scenario: AI triage system attempts to process a patient but
    // the model inference service times out. EvidenceToken cannot be produced.
    let patient_context = b"patient_id:P-2026-0042 | complaint:chest_pain | \
                            hr:145 | bp:85/50 | spo2:88 | \
                            error:MODEL_TIMEOUT_3000ms";

    println!("  [1] AI system FAILED to produce EvidenceToken");
    println!("      Error: Model inference timeout (3000ms)");
    println!("      Patient context hashed for reference\n");

    // The ContextRef captures what the system tried to process
    let failed_ctx = ContextRef::from_context(patient_context);

    // A nurse (operator) assumes accountability.
    // In production: signing_key from HSM, operator_id from SSO/LDAP.
    let authority = OperatorAuthority::new([0xFF; 32]); // HSM key
    let nurse_id = {
        let hash = blake3::hash(b"nurse:maria-santos:badge-4521");
        *hash.as_bytes()
    };
    let nurse_token = authority.issue_token(nurse_id);

    println!("  [2] OperatorToken issued for nurse Maria Santos");
    println!("      Operator ID: {}", hex::encode(nurse_id));

    // Construct the EscalatedVerdict — consumes the OperatorToken linearly.
    let verdict = EscalatedVerdict::new(
        nurse_token, // <- OperatorToken consumed here (moved, destroyed)
        Decision::Allow,
        failed_ctx,
        "AI triage system timed out. Patient presenting with chest pain, \
         tachycardia (HR 145), hypotension (85/50), hypoxia (SpO2 88%). \
         Clinical judgment: immediate treatment required. \
         Escalated per AI Act Art. 14 — human oversight."
            .to_string(),
    );

    println!("  [3] EscalatedVerdict constructed — V_esc ⊸ (O ⊗ 1) satisfied");
    println!(
        "      Decision:    {}",
        match verdict.decision() {
            Decision::Allow => "Allow (immediate treatment)",
            Decision::Deny => "Deny",
        }
    );
    println!("      Operator:    {}", verdict.operator_id_hex());
    println!("      Context ref: {}", verdict.failed_context().to_hex());
    println!("      Reason:      {}\n", verdict.reason());

    // Integrity check
    println!("  [4] Integrity check:");
    if verdict.verify_integrity() {
        println!("      PASS — HMAC seal verified\n");
    } else {
        println!("      FAIL\n");
    }

    // Demonstrate trait polymorphism
    println!("  [5] AccountableDecision trait:");
    println!("      is_automated: {}", verdict.is_automated());
    println!("      This decision is auditable but attributed to a human,");
    println!("      not to the AI system. AI Act Art. 14 satisfied.\n");

    println!("═══════════════════════════════════════════════════════════════");
    println!("  The system did NOT fail silently.");
    println!("  The system did NOT block and deny treatment.");
    println!("  The system ESCALATED with a typed, auditable trail.");
    println!("═══════════════════════════════════════════════════════════════");
}
