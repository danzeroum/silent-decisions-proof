//! # Silent Decisions Are Type Errors — Proof Library (compatibility shim)
//!
//! OS-09 (COMSI-2026-04-0112, closes H6): the repository previously carried
//! TWO divergent implementations (`paper1/src/lib.rs`, 985 lines with 17
//! proof clauses, and `btv-core/src/lib.rs`, 1,057 lines with 5), and the
//! manuscript never said which one produced Table 2. `btv-core` is the
//! single CANONICAL implementation; this crate is a compatibility re-export
//! so that `silent_decisions_proof::...` paths keep compiling.
//!
//! Canonical proof-clause suite: `btv-core` (17 clauses; see
//! `btv-core/tests/test_proof_clauses.rs` and the `#[cfg(test)]` module of
//! `btv-core/src/lib.rs`).
//!
//! Core law: `V \multimap (E \otimes C_signed)` — a `Verdict` requires
//! consuming one `EvidenceToken` and one authority-signed `ComplianceToken`.

pub use btv_core::*;
