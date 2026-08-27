//! Test 5 — Falha controlada (partição de rede / log indisponível).
//!
//! Verifica que o sistema falha fechado (fail-secure): quando o `LogSink`
//! está indisponível, NENHUM `Verdict` é emitido, e o erro é tipado
//! (`BtvError::LogUnavailable`).
//!
//! Epistemic footer:
//!   Este teste valida que `issue_verdict` retorna `Err(BtvError::LogUnavailable)`
//!   quando `sink.is_available() == false`, e que nenhum registro é appendado
//!   ao log. Ele NÃO garante que o sistema preserva Legalidade sob todas as
//!   formas de falha distribuída — apenas sob a falha explicitamente simulada
//!   (toggle do `AtomicBool`). Falhas de rede reais, partições bizantinas,
//!   ou comprometimento do backend não são cobertas.

#![cfg(test)]

use btv_core::{
    issue_verdict, BtvError, ComplianceAuthority, Decision, EvidenceToken,
    InMemoryLogSink, LogSink, SqliteLogSink,
};
use std::sync::Arc;
use std::thread;

#[test]
fn partition_in_memory_no_verdict_emitted() {
    let sink = InMemoryLogSink::new();
    sink.fail();
    assert!(!sink.is_available());

    let token = EvidenceToken::new(b"context-after-partition");
    let authority = ComplianceAuthority::new_for_test();
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();

    let result = issue_verdict(
        token, compliance, Decision::Allow,
        "post-partition attempt".to_string(), &sink,
    );

    assert!(matches!(result, Err(BtvError::LogUnavailable)),
        "must return LogUnavailable, got: {:?}", result.err());
    assert_eq!(sink.len(), 0, "no record should be appended");
}

#[test]
fn partition_sqlite_no_verdict_emitted() {
    let sink = SqliteLogSink::open_in_memory().expect("sqlite in-memory");
    sink.fail();
    assert!(!sink.is_available());

    let token = EvidenceToken::new(b"context");
    let authority = ComplianceAuthority::new_for_test();
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();

    let result = issue_verdict(
        token, compliance, Decision::Allow,
        "post-partition".to_string(), &sink,
    );

    assert!(matches!(result, Err(BtvError::LogUnavailable)));
}

#[test]
fn recovery_allows_subsequent_verdicts() {
    let sink = InMemoryLogSink::new();
    sink.fail();

    let token = EvidenceToken::new(b"first-attempt");
    let authority = ComplianceAuthority::new_for_test();
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
    let _ = issue_verdict(
        token, compliance, Decision::Allow, "first".to_string(), &sink,
    );
    assert_eq!(sink.len(), 0);

    sink.recover();
    assert!(sink.is_available());

    let token2 = EvidenceToken::new(b"second-attempt");
    let compliance2 = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
    let verdict = issue_verdict(
        token2, compliance2, Decision::Allow, "second".to_string(), &sink,
    ).expect("recovery should allow issuance");
    assert!(verdict.verify_integrity());
    assert_eq!(sink.len(), 1);
}

#[test]
fn concurrent_failures_all_rejected() {
    let sink = Arc::new(InMemoryLogSink::new());
    sink.fail();

    let authority = Arc::new(ComplianceAuthority::new_for_test());
    let mut handles = vec![];

    for i in 0..10 {
        let sink_clone = Arc::clone(&sink);
        let auth_clone = Arc::clone(&authority);
        handles.push(thread::spawn(move || {
            let token = EvidenceToken::new(format!("ctx-{i}").as_bytes());
            let compliance = auth_clone.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
            let result = issue_verdict(
                token, compliance, Decision::Allow,
                format!("concurrent-{i}"), &*sink_clone,
            );
            assert!(matches!(result, Err(BtvError::LogUnavailable)));
        }));
    }
    for h in handles {
        h.join().expect("thread panicked");
    }
    assert_eq!(sink.len(), 0);
}

#[test]
fn append_failure_also_rejected() {
    /// A LogSink that fails on append even though is_available() == true.
    struct FailOnAppendSink;
    impl LogSink for FailOnAppendSink {
        fn append(&self, _: &btv_core::VerdictRecord) -> Result<(), BtvError> {
            Err(BtvError::Backend("simulated append failure".to_string()))
        }
        fn is_available(&self) -> bool { true }
    }

    let sink = FailOnAppendSink;
    let token = EvidenceToken::new(b"context");
    let authority = ComplianceAuthority::new_for_test();
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();
    let result = issue_verdict(
        token, compliance, Decision::Allow, "test".to_string(), &sink,
    );
    assert!(matches!(result, Err(BtvError::Backend(_))));
}
