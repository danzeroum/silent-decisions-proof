//! Worked Example 2: Hiring screening under EU AI Act Art. 86
//!
//! Simulates an automated CV screening system that ranks candidates.
//! Under EU AI Act, hiring is classified as high-risk AI (Annex III).
//! Demonstrates: multi-field context, EU jurisdiction, appeal rights.

use silent_decisions_proof::{ComplianceToken, Decision, EvidenceToken, Verdict};

struct Candidate {
    id: &'static str,
    name: &'static str,
    years_experience: u32,
    skill_match_pct: f64,
    education_score: f64,
}

/// Mock screening model — weighted composite score.
fn mock_screening_model(c: &Candidate) -> f64 {
    let exp_factor = (c.years_experience as f64 / 10.0).min(1.0);
    c.skill_match_pct * 0.50 + exp_factor * 0.30 + c.education_score * 0.20
}

fn screen_candidate(c: &Candidate, cutoff: f64) -> Verdict {
    let score = mock_screening_model(c);
    let decision = if score >= cutoff { Decision::Allow } else { Decision::Deny };

    let context = format!(
        "candidate_id:{} | name:{} | years_exp:{} | skill_match:{:.2} | \
         education:{:.2} | composite_score:{:.4} | cutoff:{:.2} | \
         model:cv-screener-v2.0 | risk_class:high | \
         timestamp:2026-03-24T09:15:00Z",
        c.id, c.name, c.years_experience, c.skill_match_pct,
        c.education_score, score, cutoff
    );

    let explanation = format!(
        "Composite score {:.4} {} cutoff {:.2}. \
         Breakdown: skill match {:.0}% (weight 50%), experience {} years (weight 30%), \
         education {:.2} (weight 20%). \
         This is a high-risk AI decision under EU AI Act Annex III. \
         You have the right to contest within 30 days per Art. 86.",
        score,
        if score >= cutoff { "meets" } else { "is below" },
        cutoff,
        c.skill_match_pct * 100.0,
        c.years_experience,
        c.education_score
    );

    let token = EvidenceToken::new(context.as_bytes());
    // EU AI Act: 720 hours (30 days) appeal window for high-risk decisions
    let compliance = ComplianceToken::new("EU-AIACT-2024/1689", "1.0.0", 720);

    Verdict::new(token, compliance, decision, explanation)
}

fn main() {
    println!("=== Worked Example 2: Hiring Screening under EU AI Act ===\n");

    let candidates = vec![
        Candidate { id: "C-001", name: "Ana Garcia",        years_experience: 7,  skill_match_pct: 0.85, education_score: 0.90 },
        Candidate { id: "C-002", name: "James Chen",        years_experience: 2,  skill_match_pct: 0.60, education_score: 0.70 },
        Candidate { id: "C-003", name: "Fatima Al-Rashid",  years_experience: 12, skill_match_pct: 0.45, education_score: 0.95 },
    ];

    let cutoff = 0.65;

    for c in &candidates {
        let verdict = screen_candidate(c, cutoff);
        let status = match verdict.decision() {
            Decision::Allow => "ADVANCE",
            Decision::Deny  => "REJECT",
        };
        println!("  {} ({}): {} — integrity {}",
            c.name, c.id, status,
            if verdict.verify_integrity() { "OK" } else { "FAIL" }
        );
        println!("    Evidence:  {}", verdict.evidence_id().to_hex());
        println!("    Explain:   {}", verdict.explanation());
        println!("    Jurisdiction: {} | Appeal: {} hours\n",
            verdict.jurisdiction(), verdict.appeal_deadline_hours());
    }
}
