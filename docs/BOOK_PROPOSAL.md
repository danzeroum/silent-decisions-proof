# Book Proposal
## The Accountability Stack: A Unified Architecture for Verifiable Institutions

**Author:** Daniel Lau Pereira Soares  
**Contact:** danniellau@gmail.com  
**Target Publishers:** MIT Press (Computer Science / Law imprint),
O'Reilly Media (technical), Springer (Lecture Notes in Computer Science)

---

## The Core Argument (The Elevator Pitch)

Every time an algorithm denies you a loan, screens out your job
application, or adjusts your insurance premium, it exercises power
over your life. Yet unlike a judge, a bureaucrat, or a bank manager,
it answers to no one and leaves no verifiable trace.

*The Accountability Stack* proposes and proves that this is not an
inevitable feature of software — it is a design choice. Using tools
from programming language theory (linear types), cryptography
(zero-knowledge proofs), and political theory (Montesquieu's
Separation of Powers), the book constructs a complete architecture
for algorithmic systems that are not merely *compliant* but
*structurally just*: systems where the violation of fairness
rules is not a crime to be punished, but a compilation error that
prevents the system from running.

The book is simultaneously:
- A **technical manual** for engineers implementing accountable AI;
- A **policy framework** for regulators who want to demand structural
  guarantees rather than compliance reports; and
- A **theoretical contribution** unifying programming language theory,
  cryptography, and constitutional law into a single framework.

---

## The Audience

**Primary:**
- AI/ML engineers and architects at companies subject to EU AI Act,
  GDPR, LGPD, or comparable regulation (~500K professionals worldwide).
- AI policy researchers and regulators designing technical standards
  for algorithmic accountability.

**Secondary:**
- Computer science researchers in programming languages, formal
  methods, and distributed systems.
- Legal scholars working on the intersection of algorithmic governance
  and constitutional law.
- Graduate students in CS, law, and public policy.

**Comparable Books (and How This Differs):**

| Book | Audience | Gap this book fills |
|---|---|---|
| *Designing Data-Intensive Applications* (Kleppmann) | Engineers | No accountability layer, no legal/political theory |
| *The Alignment Problem* (Christian) | General | No formal proofs, no implementation |
| *The Black Box Society* (Pasquale) | Policy/Law | No technical solutions |
| *Weapons of Math Destruction* (O'Neil) | General | No constructive framework |

---

## Chapter Outline

### Part I — The Problem

**Chapter 1: The Silent Decision**  
The Ahmed Mohamed problem: what happens when an algorithm makes a
binding decision and no one can explain why. Real cases: Amazon hiring
(2018), Optum health (2019), COMPAS recidivism, EU credit scoring.
The anatomy of opacity: no log, no reason, no appeal.

**Chapter 2: Why "Ethics Guidelines" Are Not Enough**  
The gap between principles and enforcement. Why 127 AI ethics frameworks
have produced almost no structural change. The economics of opacity:
why companies prefer the risk of a fine to the cost of transparency
(spoiler: the math is wrong — Chapter 6 proves this).

### Part II — The Stack

**Chapter 3: The Physics of Accountability (Paper 1)**  
Linear types as accountability primitives. Why the compiler is the best
regulator. End-to-end walkthrough: building a loan decision system
where silent decisions are physically impossible. Code in Rust.

**Chapter 4: The Memory of Institutions (Paper 2)**  
Why logs lie and how to make them honest. Merkle trees, append-only
structures, and the mathematics of immutability. What Bitcoin got right
about record-keeping (and what it missed about privacy).

**Chapter 5: The Right to Be Forgotten Without the Right to Lie
(Paper 3)**  
The fundamental tension: GDPR gives you the right to erasure; AI
accountability requires permanent records. Zero-knowledge proofs as
the resolution. How to prove a system is fair without revealing a
single data point. Code in Noir.

**Chapter 6: The Economics of Honesty (Paper 4)**  
The Cost of Opacity ratio: 20 real enforcement cases. Why JPMorgan paid
$200M for a transparency failure that $5K/year of infrastructure would
have prevented. The Compliance Credit framework: turning accountability
from a cost center into a risk-mitigation asset.

### Part III — The Theory

**Chapter 7: Montesquieu in Code (Paper 5)**  
The separation of powers as a type system invariant. Formal proof that
legislative, executive, and judicial branches can be separated not by
social contract but by mathematical properties of the execution
substrate. The shift from geocentric (operator-centered) to
heliocentric (protocol-centered) institutional design.

**Chapter 8: The Living Constitution (Paper 6)**  
How institutions evolve without being captured. Stone Clauses, Sunset
Clauses, and the Tripartite Ratification requirement. Jefferson's
insight (each generation governs itself) implemented as a linear
resource type.

### Part IV — The Future

**Chapter 9: The Standard**  
How to turn this research into infrastructure. The Inclusion Receipt
RFC. How regulators can adopt Constitutional Type Schemas instead of
compliance reports. Draft language for EU AI Act amendments.

**Chapter 10: Verifiable Institutions Beyond AI**  
Banks, governments, hospitals, content moderation platforms. The
Accountability Stack as a general theory of institutional legitimacy
in the digital age. Open problems.

---

## The Underlying Research

The book is grounded in a six-paper peer-reviewed series:

| Paper | Venue (target) | Core contribution |
|---|---|---|
| P1 — Silent Decisions Are Type Errors | IEEE Computer | Linear types for AI accountability |
| P2 — BTV-Transparency | CCS / USENIX | Verifiable persistence for decision logs |
| P3 — Accountable Redaction | PoPETs 2027 | ZK proofs for privacy-preserving audit |
| P4 — Economics of Opacity | FAccT 2027 | Cost-benefit analysis, 20 real cases |
| P5 — Constitutional Code | CACM / Nature MI | Montesquieu ↔ BTV formal correspondence |
| P6 — The Living Constitution | ACM TOCS | Constitutional Amendment Protocol |

Source code, proofs, and ZK circuits are publicly available at:  
https://github.com/danzeroum/silent-decisions-proof

---

## Estimated Specifications

- **Length:** 120,000–140,000 words (approx. 350–400 pages)
- **Figures:** ~40 (architecture diagrams, proof sketches, data charts)
- **Code listings:** ~25 (Rust, Noir, pseudocode)
- **Timeline:** First draft 18 months after contract signing
- **Manuscript delivery:** Contingent on P1–P3 peer-review outcomes
  (expected Q3 2026)

---

*São Paulo, March 2026*
