//! Fail-open vs. fail-secure: the status-quo contrast experiment (Round 3,
//! Task B1 — the runtime half of the E2 baseline comparison the editorial
//! letter demands).
//!
//! The letter's E2 asks for a comparison against "a minimal baseline
//! implementation of the status quo", not only the derived economic model.
//! `benches/sweep_concurrent.rs` measures the LATENCY half
//! (`Mode::StatusQuoAsyncLog`). This file demonstrates the SEMANTIC half that
//! the latency numbers cannot show on their own:
//!
//!   Status quo  (fail-open):  decision → serialize JSON record →
//!                              fire-and-forget channel hand-off. The
//!                              decision "goes out" (the issue function
//!                              returns success) regardless of whether the
//!                              record is ever persisted. If the logging
//!                              consumer crashed, was never started, or is
//!                              permanently backlogged, nothing in the code
//!                              path notices: this is precisely the paper's
//!                              Section 1 scenario — "a system may emit a
//!                              denial, log it asynchronously, and silently
//!                              drop the log".
//!
//!   BTV         (fail-secure): a `Verdict` can only exist because an
//!                              `EvidenceToken` (BLAKE3 over the decision
//!                              context) and a `ComplianceToken` (issued by
//!                              an allowlist-validating authority) were
//!                              MOVED INTO `Verdict::new` and consumed
//!                              ((E ⊗ C) ⊸ V, Theorem 4.1). There is no
//!                              code path that materializes a decision
//!                              without the evidence having been bound to
//!                              it first. The compile-time counterpart of
//!                              this claim is the trybuild compile-fail
//!                              suite (`tests/ui/`, 8 fixtures); the
//!                              FFI boundary counterpart is
//!                              btv-python's fail-secure `BTVError` on log
//!                              sink failure.
//!
//! Deliberate implementation notes (mirroring the benchmark mode):
//!   - The send result is EXPLICITLY ignored (`let _ = sender.send(..)`) —
//!     the fire-and-forget contract. Unwrapping or propagating the error
//!     would panic/throw on a disconnected channel and fake a fail-secure
//!     behavior the status quo does not have.
//!   - The channel is thread-local to this test; the benchmark analogue
//!     keeps one pair per Rayon worker to avoid measuring shared-queue
//!     lock contention (a strawman baseline).

use std::sync::mpsc;

use btv_core::{ComplianceAuthority, Decision, EvidenceToken, Verdict};

/// The status-quo "decision issue" function: serialize a structured JSON
/// record for the decision and hand it to an async logging channel.
/// Mirrors `Mode::StatusQuoAsyncLog` in `benches/sweep_concurrent.rs`
/// (same decision data; the record embeds the full context, hex-encoded,
/// because the status quo has no compact cryptographic binding).
///
/// Returns `Ok(())` UNCONDITIONALLY: the moment this returns, the decision
/// has left the decision system. Whether the record is ever persisted is
/// not this function's concern — that is the point of the experiment.
// The Result really is always Ok — the shape models the decision API's
// contract ("issuing succeeded"), and the tests assert that unconditional
// success under record loss. clippy::unnecessary_wraps is deliberately
// overridden for that reason.
#[allow(clippy::unnecessary_wraps)]
fn status_quo_issue_decision(context: &[u8], tx: &mpsc::Sender<String>) -> Result<(), ()> {
    let ts_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0u64, |d| u64::try_from(d.as_millis()).unwrap_or_default());
    let record = serde_json::json!({
        "ts_unix_ms": ts_unix_ms,
        "decision": "deny",
        "jurisdiction": "EU-GDPR",
        "policy_version": "contrast-test-1",
        "context_hex": hex::encode(context),
        "appeal_deadline_hours": 720,
        "explanation": "status-quo-contrast",
    })
    .to_string();
    // Fire-and-forget: the send error (disconnected or full) is swallowed
    // ON PURPOSE. This single line is the fail-open semantics under test.
    let _ = tx.send(record);
    Ok(())
}

/// Scenario A — the logging consumer is GONE before the decision is made
/// (crashed, restarted, never deployed). The status quo still issues the
/// decision: `status_quo_issue_decision` returns success even though the
/// record was destroyed inside the discarded `SendError`.
#[test]
fn fail_open_dead_logger_still_issues_decision() {
    let (tx, rx) = mpsc::channel::<String>();
    // The consumer died before the decision. Dropping the Receiver is the
    // canonical "logger unavailable" state.
    drop(rx);

    let outcome = status_quo_issue_decision(b"subject:alice|score:0.42", &tx);

    // The decision "went out". No error, no retry, no backpressure signal.
    assert!(outcome.is_ok());
    // And nothing was persisted: there is no consumer to persist it. The
    // serialized record existed only inside the ignored SendError and was
    // destroyed with it.
}

/// Scenario B — the logging consumer is alive but NEVER drains the queue
/// (backlogged, misconfigured, blocked on a slow SIEM forwarder). Every
/// decision is "issued" successfully; the records accumulate without any
/// persistence guarantee; when the consumer is eventually torn down, every
/// undrained record is destroyed. This is "omittable under load" from the
/// paper's Section 1.
#[test]
fn fail_open_backlogged_logger_loses_records_silently() {
    const N: usize = 1000;
    let (tx, rx) = mpsc::channel::<String>();

    for i in 0..N {
        let context = format!("subject:alice|iteration:{i}");
        let outcome = status_quo_issue_decision(context.as_bytes(), &tx);
        // Every single decision reports success...
        assert!(outcome.is_ok());
    }

    // ...while the records merely sit in an undrained queue. The emitter
    // has no way to distinguish "persisted" from "parked" from "dropped".
    // The consumer is now torn down WITHOUT ever draining:
    drop(rx);

    // N decisions were issued; ZERO records were persisted; no error was
    // raised anywhere in the pipeline. The records are unrecoverable —
    // they were destroyed with the receiver.
}

/// The structured record is a well-formed JSON decision event with the
/// fields a downstream SIEM would need — this keeps the baseline honest
/// (a status quo that logged nothing would be a strawman in the OTHER
/// direction).
#[test]
fn status_quo_record_is_wellformed_json() {
    let (tx, rx) = mpsc::channel::<String>();
    status_quo_issue_decision(b"subject:alice|score:0.42", &tx).expect("always Ok");

    let record = rx
        .try_recv()
        .expect("record is enqueued while the receiver lives");
    let parsed: serde_json::Value = serde_json::from_str(&record).expect("record is valid JSON");

    assert_eq!(parsed["decision"], "deny");
    assert_eq!(parsed["jurisdiction"], "EU-GDPR");
    assert_eq!(parsed["policy_version"], "contrast-test-1");
    assert_eq!(parsed["appeal_deadline_hours"], 720);
    assert_eq!(parsed["explanation"], "status-quo-contrast");
    // Full context embedded (hex) — exactly twice the context's byte
    // length: "subject:alice|score:0.42" is 24 bytes -> 48 hex chars.
    assert_eq!(parsed["context_hex"].as_str().map(str::len), Some(48));
}

/// The BTV side of the contrast (fail-secure): a `Verdict` exists ONLY as
/// the result of consuming an `EvidenceToken` (BLAKE3 over the context)
/// and a `ComplianceToken` (authority-issued, allowlist-validated).
///
/// Where the status quo returned `Ok(())` with the record already lost,
/// here the decision artifact carries its own evidence binding: the
/// verdict's `evidence_id()` is the BLAKE3 digest of exactly the context
/// bytes passed in, verifiable after the fact via `verify_integrity()`.
/// There is no "issue now, maybe persist later" path to demonstrate —
/// the type system removed it (see `tests/ui/` for the compile-fail
/// proofs: dropped tokens, token reuse, external construction).
#[test]
fn fail_secure_bt_verdict_requires_consumed_tokens() {
    let authority = ComplianceAuthority::new_for_test();

    let context = b"subject:alice|score:0.42|threshold:0.50";

    // The only route to a Verdict consumes both tokens by value.
    let evidence = EvidenceToken::new(context);
    let compliance = authority
        .issue_token("EU-GDPR", "contrast-test-1", 720)
        .expect("EU-GDPR is allowlisted");
    let verdict = Verdict::new(
        evidence,
        compliance,
        Decision::Deny,
        "status-quo-contrast".to_string(),
    )
    .expect("new_for_test authority holds the recognized key");

    // The decision artifact is self-evidencing: the digest it carries is
    // the BLAKE3 hash of the exact context above, and it re-verifies.
    assert_eq!(verdict.decision(), &Decision::Deny);
    assert_eq!(verdict.jurisdiction(), "EU-GDPR");
    assert_eq!(verdict.policy_version(), "contrast-test-1");
    assert_eq!(verdict.appeal_deadline_hours(), 720);
    assert_eq!(
        verdict.evidence_id().to_hex(),
        // The same digest an independent re-hash of the context produces.
        // (Blake3Hash::of is pub(crate) by design — one of the crate's
        // protections — so the test re-computes it with the algorithm
        // directly instead of going through the API under test. blake3's
        // to_hex() returns an ArrayString<64>, hence the as_str() here.)
        blake3::hash(context).to_hex().as_str()
    );
    assert!(verdict.verify_integrity());

    // Contrast, stated once more for the record: in the two fail-open
    // scenarios above, the equivalent assertions would be about a record
    // that no longer exists. Here the assertion is about an artifact that
    // cannot have come into being without its evidence.
}
