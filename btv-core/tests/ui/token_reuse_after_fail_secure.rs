// OS-06 gate — token non-reuse after the fail-secure path is COMPILE-level.
//
// After `issue_verdict` returns Err(LogUnavailable), the caller cannot
// retry with the same tokens: they were MOVED into the function, so a
// second call using them is a type error (E0382, use of moved value).
//
// Audit note (F6): the pre-audit code justified `mem::forget` with the
// claim that returning the tokens would allow retry. The claim was
// inverted — move semantics already deny retry, and `mem::forget` on
// non-Drop types merely leaked their allocations (clippy::forget_non_drop).
// The fix replaced `forget` with `drop`; this trybuild test pins the
// actual linearity guarantee at the API surface.
#![deny(unused_must_use)]

use btv_core::{
    issue_verdict, ComplianceAuthority, Decision, EvidenceToken, InMemoryLogSink,
};

fn main() {
    let sink = InMemoryLogSink::new();
    sink.fail();
    let authority = ComplianceAuthority::new_from_env();
    let token = EvidenceToken::new(b"ctx");
    let compliance = authority.issue_token("BR-LGPD", "1.0.0", 720).unwrap();

    // First call: fail-secure rejection (Err), tokens consumed by move.
    let _ = issue_verdict(token, compliance, Decision::Allow, "x".to_string(), &sink);

    // Retry attempt: `token` and `compliance` no longer exist in this scope.
    // Expected: error[E0382]: use of moved value: `token`
    let _ = issue_verdict(token, compliance, Decision::Allow, "x".to_string(), &sink);
}
