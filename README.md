# The Accountability Stack
## A Unified Architecture for Verifiable Institutions

> *"The violation of the constitution is not a crime to be punished.  
> It is a compilation error that prevents the institution from ever existing."*
> — Paper 5

This repository contains the source code, proofs, circuits, and LaTeX
source for a six-paper series on algorithmic accountability. Together,
they form a complete, layered architecture — the **Accountability Stack** —
for building AI systems that are not merely compliant, but
**structurally just**.

---

## The Stack at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│  P6  The Living Constitution   │ Evolution without capture       │
│      Amendment Protocol        │ Change ⊸ Consensus ⊗ Legitimacy │
├─────────────────────────────────────────────────────────────────┤
│  P5  Constitutional Code       │ Separation of Powers in code    │
│      Montesquieu ↔ BTV         │ P_L ∩ P_E ∩ P_J = ∅            │
├─────────────────────────────────────────────────────────────────┤
│  P4  Economics of Opacity      │ CoO ratio: fines vs infra cost  │
│      20 enforcement cases      │ TCO(N) < ρ · N                  │
├─────────────────────────────────────────────────────────────────┤
│  P3  Accountable Redaction     │ Privacy-preserving audit        │
│      ZK proofs in Noir         │ Redaction ⟺ π_auth ∧ π_stat    │
├─────────────────────────────────────────────────────────────────┤
│  P2  BTV-Transparency          │ Append-only verifiable log      │
│      Merkle persistence        │ Delivery ⊸ Verdict ⊗ Receipt    │
├─────────────────────────────────────────────────────────────────┤
│  P1  Silent Decisions          │ Evidence at compile time        │
│      Linear types in Rust      │ Verdict ⊸ Evidence ⊗ Cert       │
└─────────────────────────────────────────────────────────────────┘
```

---

## End-to-End Data Flow

This is what happens when an AI system makes a single decision under
the full Accountability Stack:

```
[1] INPUT
    User submits credit application → raw features arrive at Operator

[2] TYPE CHECK (P1 — Linear Types)
    btv-core type system verifies:
      EvidenceToken is consumed exactly once
      ComplianceCertificate is produced
    ┌──────────────────────────────────────┐
    │  fn decide(e: EvidenceToken,         │
    │            rules: &ConstitType)      │
    │  -> (Verdict, ComplianceCert) { .. } │
    └──────────────────────────────────────┘
    ↓ compile-time error if evidence is missing or duplicated

[3] LOG COMMIT (P2 — Merkle Persistence)
    Verdict + Cert → LogServer.append()
    LogServer returns: InclusionReceipt { leaf_hash, merkle_path, root }
    Root is published to public bulletin board
    ┌──────────────────────────────────────┐
    │  DeliveryToken::seal(receipt)        │
    │  // only valid with InclusionReceipt │
    └──────────────────────────────────────┘
    ↓ decision cannot be delivered without committed receipt

[4] DELIVERY
    DeliveryToken::send() → external API response to user
    Operator receives: denial/approval + opaque receipt ID

[5] AUDIT (P3 — ZK Proofs)
    Auditor requests batch proof for epoch T
    Operator generates π_stat: ZK proof that
      approval_rate(group_A) / approval_rate(group_B) ∈ [1-ε, 1+ε]
    Auditor verifies: ZKVerify(π_stat, stmt) = 1
    ↓ auditor learns: system is fair. Learns nothing else.

[6] ECONOMIC SIGNAL (P4)
    Compliance Credit δ = 0.9 (Level 2 Constitutional)
    Expected fine liability reduced by factor (1 - δ)
    TCO_annual ≈ $5,000–$50,000 depending on decision volume

[7] CONSTITUTIONAL LAYER (P5)
    The type system = Legislative branch  (defines the rules)
    The operator    = Executive branch    (executes, cannot hide)
    The auditor     = Judicial branch     (verifies, no permission needed)
    P_L ∩ P_E ∩ P_J = ∅  ← Constitutional Completeness

[8] AMENDMENT (P6)
    Legislative mandate M = Mandate[L_v, t_exp]
    Policy updates: signed by L alone
    Stone Clause changes: require σ_L ∧ σ_J ∧ σ_E_rep
    At t > t_exp: Verdict::new() fails → Constitutional Interregnum
```

---

## Repository Structure

```
silent-decisions-proof/
├── paper1/          Silent Decisions Are Type Errors (IEEE Computer)
│   ├── main.tex
│   ├── section*.tex
│   └── refs.bib
├── paper2/          BTV-Transparency (CCS/USENIX)
│   ├── main.tex
│   └── ...
├── paper3/          Accountable Redaction (PoPETs 2027)
│   ├── main.tex
│   ├── circuits/    Noir ZK circuits
│   └── ...          CI: 697ms proof, 16KB, verified
├── paper4/          Economics of Opacity (FAccT 2027)
│   ├── main.tex
│   └── ...          20 enforcement cases, CoO analysis
├── paper5/          Constitutional Code (CACM / Nature MI)
│   ├── main.tex
│   └── ...          3 independence theorems + completeness corollary
├── paper6/          The Living Constitution (ACM TOCS / JCPE)
│   ├── main.tex
│   └── ...          Amendment Soundness + Mandatory Renewal
├── docs/
│   ├── BOOK_PROPOSAL.md          MIT Press / O'Reilly proposal
│   └── draft-soares-btv-inclusion-receipt-00.txt   IETF Internet Draft
├── ROADMAP.md
└── README.md        ← you are here
```

---

## Submission Status

| Paper | Venue | Deadline | Status |
|---|---|---|---|
| P1 | IEEE Computer | 13 Apr 2026 | ✅ Ready |
| P2 | CCS / USENIX SEC | TBD (CFP) | ✅ Ready |
| P3 | PoPETs 2027 | Jan 2027 | ✅ CI green |
| P4 | FAccT 2027 | Jan 2027 | ⚠️ Verify fine amounts |
| P5 | CACM / Nature MI | Rolling | ✅ Ready |
| P6 | ACM TOCS / JCPE | Rolling | ✅ Ready |

---

## How to Cite

Until papers are published, cite the series as:

```bibtex
@misc{btv-stack-2026,
  author = {Soares, Daniel Lau Pereira},
  title  = {The Accountability Stack: Source Repository},
  year   = {2026},
  url    = {https://github.com/danzeroum/silent-decisions-proof}
}
```

---

*From bits to Montesquieu to Jefferson — São Paulo, March 2026.*
