//! # Silent Decisions Are Type Errors — Proof Library
//!
//! OS-09 (COMSI-2026-04-0112, closes H4/H6): this crate used to carry its
//! **own** independent copy of every BTV type (`Verdict`, `ComplianceToken`,
//! `EscalatedVerdict`, ...), diverging from `btv-core` over four audit
//! rounds — most visibly, this copy never received the OS-02 signing fix
//! (`ComplianceAuthority.signing_key` was still dead code here), and the
//! manuscript never said which of the two implementations produced its
//! Table 2 numbers, because there was no way to say: they disagreed.
//!
//! `btv-core` is now the single canonical reference implementation. This
//! crate re-exports it rather than shadow it, so:
//!
//! - There is exactly one implementation to audit, one place a fix such as
//!   OS-01..OS-08 needs to land, and one clause suite
//!   (`grep -c "fn clause_" btv-core/src/lib.rs` = 17, matching §1/§7 of
//!   the manuscript).
//! - This crate's binary (`src/main.rs`) and worked examples
//!   (`examples/*.rs`) keep running unmodified in spirit — only the
//!   `Verdict::new` call sites gained the `.expect(...)` that
//!   `btv-core`'s signature-verifying constructor now requires (OS-02
//!   made it fallible: a forged or foreign-authority `ComplianceToken` is
//!   rejected instead of silently accepted).
//! - `paper1/benches/verdict_construction.rs` is superseded by
//!   `btv-core/benches/verdict_construction.rs` (OS-07's durable-vs-RAM
//!   contrast) and by `btv-core/benches/sweep_concurrent.rs` (OS-07's
//!   5-mode concurrency sweep) as the source of any number cited in the
//!   manuscript; it is kept here only as a historical artifact of the
//!   original submission and is not cited by §5.
//!
//! See `btv-core/src/lib.rs` for the actual type definitions, the 17
//! proof clauses, and their doc comments.

pub use btv_core::*;
