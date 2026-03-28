//! Worked Example 1: Credit scoring denial under LGPD
//!
//! Simulates a credit-scoring API that receives an application,
//! evaluates it via a mock ML model, and produces a BTV Verdict.
//! Demonstrates: real-world payload → EvidenceToken → Verdict → audit.

use silent_decisions_proof::{ComplianceToken, Decision, EvidenceToken, Verdict};

/// Mock ML model — returns a credit score between 0.0 and 1.0.
fn mock_credit_model(income: f64, debt: f64, history_months: u32) -> f64 {
    // Simplified logistic: higher income and longer history → higher score
    let ratio = if income > 0.0 { debt / income } else { 1.0 };
    let history_factor = (history_months as f64 / 120.0).min(1.0);
    (1.0 - ratio) * 0.6 + history_factor * 0.4
}

fn main() {
    println!("=== Worked Example 1: Credit Scoring under LGPD ===\n");

    // --- Incoming request (simulates API payload) ---
    let applicant = "maria.silva@example.com";
    let income = 4500.00_f64;     // BRL/month
    let debt = 3200.00_f64;       // BRL total outstanding
    let history_months = 18_u32;
    let threshold = 0.50_f64;

    // --- Model inference ---
    let score = mock_credit_model(income, debt, history_months);
    let decision_outcome = if score >= threshold {
        Decision::Allow
    } else {
        Decision::Deny
    };

    // --- Construct the decision context (what the AI saw) ---
    let context = format!(
        "applicant:{} | income:{:.2} | debt:{:.2} | history_months:{} | \
         model:credit-scorer-v3.1 | score:{:.4} | threshold:{:.2} | \
         timestamp:2026-03-24T14:32:00Z",
        applicant, income, debt, history_months, score, threshold
    );

    // --- BTV: create evidence from the FULL decision context ---
    let token = EvidenceToken::new(context.as_bytes());

    // --- BTV: compliance metadata per LGPD Art. 18§2 ---
    let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 720); // 30 days

    // --- BTV: construct the Verdict (consumes both tokens atomically) ---
    let explanation = format!(
        "Credit score {:.4} is below the required threshold of {:.2}. \
         Basis: debt-to-income ratio {:.2}, credit history {} months. \
         You may contest this decision within 30 days per LGPD Art. 18§2.",
        score, threshold, debt / income, history_months
    );

    let verdict = Verdict::new(token, compliance, decision_outcome, explanation);

    // --- Output: what an auditor or the applicant would see ---
    println!("  Applicant:    {}", applicant);
    println!("  Score:        {:.4}", score);
    println!("  Threshold:    {:.2}", threshold);
    println!("  Decision:     {:?}", match verdict.decision() {
        Decision::Allow => "APPROVED",
        Decision::Deny  => "DENIED",
    });
    println!("  Evidence ID:  {}", verdict.evidence_id().to_hex());
    println!("  Explanation:  {}", verdict.explanation());
    println!("  Jurisdiction: {}", verdict.jurisdiction());
    println!("  Appeal window: {} hours", verdict.appeal_deadline_hours());
    println!("  Integrity:    {}\n", if verdict.verify_integrity() { "PASS" } else { "FAIL" });

    // --- The context hash is deterministic: same input → same evidence ---
    // An auditor can re-hash the stored context and verify it matches evidence_id.
    let rehash = EvidenceToken::new(context.as_bytes());
    let rehash_verdict = Verdict::new(
        rehash,
        ComplianceToken::new("BR-LGPD", "1.0.0", 720),
        Decision::Deny,
        verdict.explanation().to_string(),
    );
    println!("  Reproducibility check:");
    println!("    Original evidence:  {}", verdict.evidence_id().to_hex());
    println!("    Re-hashed evidence: {}", rehash_verdict.evidence_id().to_hex());
    assert_eq!(
        verdict.evidence_id().to_hex(),
        rehash_verdict.evidence_id().to_hex(),
        "Same context must produce same evidence hash"
    );
    println!("    Match: YES — evidence is reproducible\n");
}
