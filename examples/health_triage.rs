//! Worked Example 3: Emergency triage prioritization
//!
//! Simulates an AI-assisted triage system that assigns priority levels.
//! Demonstrates: healthcare domain, LGPD compliance, high-stakes Allow
//! decisions also require evidence (not just denials).

use silent_decisions_proof::{ComplianceToken, Decision, EvidenceToken, Verdict};

struct Patient {
    id: &'static str,
    chief_complaint: &'static str,
    heart_rate: u32,
    systolic_bp: u32,
    spo2: u32,       // oxygen saturation %
    pain_scale: u32, // 0-10
}

#[derive(Debug)]
enum TriageLevel {
    Immediate, // Red — life-threatening
    Urgent,    // Orange — serious but stable
    Standard,  // Green — can wait
}

/// Mock triage model — severity scoring based on vitals.
fn mock_triage_model(p: &Patient) -> (TriageLevel, f64) {
    let mut severity = 0.0_f64;

    // SpO2 < 90% is critical
    if p.spo2 < 90 { severity += 0.40; }
    else if p.spo2 < 95 { severity += 0.20; }

    // Heart rate extremes
    if p.heart_rate > 120 || p.heart_rate < 50 { severity += 0.25; }
    else if p.heart_rate > 100 { severity += 0.10; }

    // Blood pressure
    if p.systolic_bp < 90 || p.systolic_bp > 180 { severity += 0.20; }

    // Pain
    severity += (p.pain_scale as f64 / 10.0) * 0.15;

    let level = if severity >= 0.60 {
        TriageLevel::Immediate
    } else if severity >= 0.30 {
        TriageLevel::Urgent
    } else {
        TriageLevel::Standard
    };

    (level, severity)
}

fn triage_patient(p: &Patient) -> Verdict {
    let (level, severity) = mock_triage_model(p);

    // In triage, both Allow (prioritize) and Deny (deprioritize) carry consequences
    let decision = match level {
        TriageLevel::Immediate | TriageLevel::Urgent => Decision::Allow,
        TriageLevel::Standard => Decision::Deny,
    };

    let context = format!(
        "patient_id:{} | complaint:{} | hr:{} | bp:{} | spo2:{} | \
         pain:{} | severity:{:.4} | triage:{:?} | \
         model:triage-assist-v1.2 | timestamp:2026-03-24T22:45:00Z",
        p.id, p.chief_complaint, p.heart_rate, p.systolic_bp,
        p.spo2, p.pain_scale, severity, level
    );

    let explanation = format!(
        "Triage level: {:?} (severity {:.2}). Vitals: HR {} bpm, BP {}, SpO2 {}%, \
         pain {}/10. Chief complaint: {}. \
         This assessment may be reviewed by the attending physician. \
         Patient or legal guardian may request full explanation per LGPD Art. 18§2.",
        level, severity, p.heart_rate, p.systolic_bp, p.spo2,
        p.pain_scale, p.chief_complaint
    );

    let token = EvidenceToken::new(context.as_bytes());
    // Healthcare: 168 hours (7 days) initial review window
    let compliance = ComplianceToken::new("BR-LGPD", "1.0.0", 168);

    Verdict::new(token, compliance, decision, explanation)
}

fn main() {
    println!("=== Worked Example 3: Emergency Triage ===\n");

    let patients = vec![
        Patient { id: "P-4401", chief_complaint: "chest pain, diaphoresis",
                  heart_rate: 135, systolic_bp: 85, spo2: 88, pain_scale: 9 },
        Patient { id: "P-4402", chief_complaint: "ankle sprain",
                  heart_rate: 78, systolic_bp: 125, spo2: 98, pain_scale: 5 },
        Patient { id: "P-4403", chief_complaint: "shortness of breath",
                  heart_rate: 110, systolic_bp: 145, spo2: 91, pain_scale: 6 },
    ];

    for p in &patients {
        let verdict = triage_patient(p);
        let action = match verdict.decision() {
            Decision::Allow => "PRIORITIZE",
            Decision::Deny  => "STANDARD QUEUE",
        };
        println!("  {} — {} — integrity {}",
            p.id, action,
            if verdict.verify_integrity() { "OK" } else { "FAIL" }
        );
        println!("    Evidence: {}", verdict.evidence_id().to_hex());
        println!("    {}\n", verdict.explanation());
    }

    println!("  Note: all three decisions — including PRIORITIZE — carry");
    println!("  non-repudiable evidence. The BTV invariant applies equally");
    println!("  to Allow and Deny outcomes.");
}
