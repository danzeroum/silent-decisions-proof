
╔══════════════════════════════════════════════════════════════════════════════╗
║           HANDOFF — SEÇÃO 1: INTRODUCTION                                  ║
║           Paper: Silent Decisions Are Type Errors                          ║
║           IEEE Computer — Deadline: 13 April 2026                         ║
╚══════════════════════════════════════════════════════════════════════════════╝

Para: Analista responsável pela Seção 1
De:   Sessão de destilação (24 mar 2026)
Ref:  github.com/danzeroum/BuildToValueGovernance
      github.com/danzeroum/silent-decisions-proof (novo — prova isolada)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 1 — O QUE É ESTE PAPER (contexto completo)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

TÍTULO DEFINITIVO:
  "Silent Decisions Are Type Errors: Enforcing AI Accountability
   via Linear Resource Types"

SUBTÍTULO DE TRABALHO (não vai no paper):
  "Aplicando Lógica Linear de Girard (1987) ao problema de
   decisões de IA sem rastro forense"

VENUE: IEEE Computer — Special Issue on AI Governance
FORMATO: Magazine article, 5.000–7.000 palavras
OPEN SCIENCE: github.com/danzeroum/silent-decisions-proof
AUTORES: Daniel Lau Pereira Soares

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 2 — A INOVAÇÃO (o que a introdução deve vender)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

LEI FUNDAMENTAL DO PAPER:

    V ⊸ (E ⊗ C)

  V = Verdict (decisão de IA com consequência jurídica)
  E = EvidenceToken (rastro forense, BLAKE3)
  C = ComplianceToken (direitos do afetado: contestação, explicação)
  ⊸ = implicação linear (Girard 1987): consumo único, não duplicável
  ⊗ = produto tensorial linear: E e C são consumidos JUNTOS, atomicamente

FRASE ÂNCORA (deve aparecer no abstract e na intro):
  "A silent decision is not an illegal act — it is a type error."

O QUE ISSO SIGNIFICA PARA O LEITOR IEEE:
  - Sistemas atuais (OPA, Kyverno, logs assíncronos): uma decisão de IA
    pode existir SEM evidência. O rastro é criado depois, separadamente,
    e pode ser omitido ou adulterado.
  - BTV via Linear Types: um Verdict NÃO COMPILA sem consumir
    exatamente um EvidenceToken e um ComplianceToken.
    A garantia é ESTÁTICA (compile time), não dinâmica (runtime).

ANALOGIA PEDAGÓGICA CENTRAL (use na intro, é poderosa para IEEE):
  "assert(evidence != null)" está para BTV linear types
  assim como
  "free() com cuidado em C" está para "ownership em Rust".

  Um é segurança por DISCIPLINA (evitável, esquecível, desligável).
  O outro é segurança por DESIGN (impossível de bypassar sem reescrever o kernel).

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 3 — CONTRIBUIÇÕES DECLARADAS (exatamente estas — não adicionar, não remover)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

C1 — Linear Resource Types para Accountability
  Primeiro framework que codifica obrigações regulatórias de IA
  (LGPD Art. 18§2, EU AI Act Art. 86) como recursos lineares
  no sentido de Girard (1987). Um Verdict que não carrega evidência
  e compliance é um erro de tipo, não um log ausente.

C2 — Decision-Evidence Causal Binding
  EvidenceToken é um recurso linear (#[must_use], não-Clone, não-Copy).
  A única forma de obter um Blake3Hash para um Verdict é via
  EvidenceToken::consume(), que MOVE e DESTRÓI o token.
  Garante: toda decisão materializada tem uma e exatamente uma
  evidência causalmente anterior.

C5 — Teorema do Enclausuramento Constitucional (com prova)
  Theorem: In any system satisfying the BTV type invariant,
  the set of materialized silent decisions is empty.
  Prova: por contradição via sistema de tipos Rust (ver repositório).
  Machine-checkable: `cargo test` no companion repo verifica as
  6 cláusulas da prova.

NÃO ESTÃO NESTE PAPER (foram deliberadamente removidos):
  ✗ Análise comparativa LGPD × EU AI Act × NIST (→ paper futuro P1-orig)
  ✗ Dynamic Bias Budgeting / Pessimistic Bias Credit (→ AIES 2026)
  ✗ Análise econômica / Nash equilibrium (→ FAccT 2027)
  ✗ PolicyElicitor, NegotiationEngine, STRIDE (→ ARIA — confidencial)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 4 — O QUE O PAPER NÃO AFIRMA (crítico para honestidade IEEE)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

O paper NÃO afirma:
  ✗ Que as explicações geradas são semanticamente verdadeiras
    (isso é AI alignment — problema em aberto)
  ✗ Que a decisão de IA é correta (problema separado)
  ✗ Que compliance metadata é precisa (problema de policy authoring)
  ✗ Que "silent decisions" são eliminadas no mundo real —
    apenas no sistema que implementa o tipo invariante

O paper AFIRMA:
  ✓ Que no BTV, silent decisions são compile-time impossíveis
  ✓ Que toda decisão materializada consumiu exatamente 1 EvidenceToken
  ✓ Que adulteração pós-fato é detectável via verify_integrity()
  ✓ Que o overhead de 37.8ms p99 é o custo previsível dessa garantia

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 5 — TRABALHO ANTERIOR (o que a intro CITA como prior work)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

CITAR (não reusar texto):
  [Soares 2026a] — "BuildToValue: Arquitetura de Governança Verificável..."
                   SBSI 2026 (rejected) — descreve a arquitetura BTV geral,
                   latência P99=37.8ms, cenários LGPD.
                   → Citar como: "We build on the BTV architecture [Soares 2026a]"

  [Soares 2026b] — "Separating Perception from Judgment..."
                   ICLR 2026 AIWILD (rejected, OpenReview público CC BY 4.0)
                   → Descreve a separação Rust/Python, Mercy Algorithm.
                   → Citar como: "The hybrid kernel architecture is described in [Soares 2026b]"

NÃO MENCIONAR:
  ✗ ARIA Scaling Trust (confidencial — aguardando £200k)
  ✗ AI4PG / Gitcoin (confidencial — aguardando $10k)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 6 — FERRAMENTAS CONCORRENTES (relacionadas à intro)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

IMPORTANTE: a diferença não é "BTV faz o que OPA não faz".
A diferença é de CATEGORIA — eles governam coisas diferentes:

  OPA / Kyverno / Checkov / Cloud Custodian:
    → Governam RECURSOS DE INFRAESTRUTURA (pods, buckets, configs)
    → Decisão = booleano (allow/deny)
    → Falta de evidência → log assíncrono (pode ser ignorado, omitido)
    → Fail-open no timeout (padrão de mercado)
    → Sem conceito de "pessoa afetada" ou "direito de contestação"

  BTV (este paper):
    → Governa ATOS COM CONSEQUÊNCIAS JURÍDICAS SOBRE PESSOAS
      (crédito, emprego, saúde, concessão de benefício)
    → Decisão = Verdict (tipo composto, compile-time verificado)
    → Falta de evidência → ERRO DE TIPO (não compila)
    → Fail-secure (qualquer erro → BLOCK)
    → Contestability deadline é campo obrigatório do tipo

  Guardrails AI / NeMo Guardrails / LlamaGuard:
    → Filtram INPUT/OUTPUT de LLMs (conteúdo inapropriado)
    → Não produzem evidência forense
    → Não têm contestabilidade
    → Não são type-safe

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 7 — ESTRUTURA DO PAPER COMPLETO (contexto para a intro)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

O paper tem 6 seções. A intro é a Seção 1. As demais:

  Seção 2 — Background (500 palavras)
    Linear Logic (Girard 1987), Rust ownership como Linear Types,
    por que OPA/Kyverno não resolve.

  Seção 3 — The BTV Type System (1.200 palavras)
    Definição formal de EvidenceToken, ComplianceToken, Verdict.
    V ⊸ (E ⊗ C). Código Rust compilável.

  Seção 4 — Constitutional Enclosure Theorem (800 palavras)
    Definição 1: Decisão Materializada
    Definição 2: Decisão Silenciosa
    Teorema + Prova formal (15 linhas, por contradição)
    Corolário: fail-secure como propriedade derivada

  Seção 5 — Implementation (1.200 palavras)
    Arquitetura BTV (citar Soares 2026a/b)
    Benchmark: 37.8ms p99, 9.632 bytes fixed evidence
    github.com/danzeroum/silent-decisions-proof

  Seção 6 — Discussion + Related Work (700 palavras)
    Por que não foi feito antes
    Limitações honestas
    Economia como motivação (100 palavras, sem números fictícios)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 8 — ESPECIFICAÇÃO DA SEÇÃO 1 (o que você precisa escrever)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

COMPRIMENTO: 500–650 palavras
FORMATO: IEEE Computer Magazine (parágrafos corridos, sem subseções na intro)
TOM: acadêmico-técnico, sem hype. Active voice. Direto.

ESTRUTURA OBRIGATÓRIA (4 parágrafos):

  § PARÁGRAFO 1 — O Problema: Silent Decisions (150 palavras)
    - AI agents tomam decisões com consequências jurídicas em ms
    - Exemplos concretos: crédito negado, vaga não oferecida, benefício bloqueado
    - O sistema atual: decisão existe, log pode ou não existir
    - Definir "silent decision" claramente aqui:
      "A silent decision is an AI-generated act with legal consequences
       for an individual that carries no non-repudiable evidence chain."
    - Por que isso importa: LGPD Art. 18§2, EU AI Act Art. 86 exigem
      explicação + contestação — mas tecnicamente não impedem a decisão
      de existir sem elas.

  § PARÁGRAFO 2 — Insuficiência das Soluções Atuais (150 palavras)
    - OPA/Kyverno: governam infraestrutura, não atos sobre pessoas
    - Logs assíncronos: subproduto separável, omissível, adulterável
    - assert(evidence != null): runtime, desligável, esquecível
    - A lacuna central: "No existing framework makes the absence of
      evidence a compile-time error rather than a runtime omission."
    - Reforçar a analogia C/Rust: disciplina vs. design

  § PARÁGRAFO 3 — A Solução: V ⊸ (E ⊗ C) (150 palavras)
    - Introduzir Linear Logic (Girard 1987) em 2 frases
    - A ideia central: tratar evidência e compliance como recursos lineares
    - Um Verdict só existe se CONSUMIR EvidenceToken ⊗ ComplianceToken
    - Isso é diferente de ∧ (conjunção clássica): recursos independentes
      vs. recursos consumidos juntos, atomicamente
    - Consequência: silent decisions tornam-se type errors

  § PARÁGRAFO 4 — Contribuições e Estrutura do Paper (100 palavras)
    "This paper makes three contributions:
     (1) [C1] A formal encoding of AI accountability obligations as
         linear resource types, establishing V ⊸ (E ⊗ C) as the
         core type invariant;
     (2) [C2] Decision-Evidence Causal Binding via linear token
         consumption, proving that each materialized Verdict consumed
         exactly one causally-prior EvidenceToken;
     (3) [C5] The Constitutional Enclosure Theorem: a formal proof
         that, in any system satisfying the type invariant, the class
         of silent decisions is empty — machine-checkable via
         `cargo test` on the companion repository."
    Fechar com: estrutura das seções restantes (1 frase).

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 9 — REGRAS DE ESCRITA (anti-padrões a evitar)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

PROIBIDO:
  ✗ "We propose a novel framework..." (clichê, todo paper faz isso)
  ✗ "AI is increasingly important..." (trivial, não diz nada)
  ✗ "Our approach is better than existing solutions" (prove, não afirme)
  ✗ Qualquer número sem fonte verificável (ρ, M, % fictícios)
  ✗ Mencionar ARIA ou AI4PG (confidencial)
  ✗ Reusar texto de Soares 2026a/b (citar, não copiar)
  ✗ Usar "prove" sem prova no paper ("we demonstrate" é mais seguro)
  ✗ Qualquer promessa que o resto do paper não entrega

OBRIGATÓRIO:
  ✓ Definir "silent decision" explicitamente (Parágrafo 1)
  ✓ Mencionar Linear Logic e Girard (1987) (Parágrafo 3)
  ✓ A frase: "...is a type error, not a runtime omission" (Parágrafo 3)
  ✓ A analogia C/Rust: disciplina vs. design (Parágrafo 2)
  ✓ Listar as 3 contribuições como bullets numerados (Parágrafo 4)
  ✓ Tom: um revisor sênior que já viu 1.000 papers de "IA ética"
    deve ler o primeiro parágrafo e pensar: "isso é diferente"

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PARTE 10 — REFERÊNCIAS NECESSÁRIAS NA SEÇÃO 1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

OBRIGATÓRIAS na intro:
  [1] Girard, J.-Y. "Linear Logic." Theoretical Computer Science, 1987.
      → fundação teórica do ⊗ e ⊸
  [2] Wadler, P. "Is there a use for linear logic?" ACM PEPM, 1991.
      → ponte entre Linear Logic e linguagens de programação
  [3] Brazil. LGPD, Law 13.709/2018, Art. 18§2.
      → motivação regulatória
  [4] EU. Regulation 2024/1689 (EU AI Act), Art. 86.
      → motivação regulatória
  [5] Soares, D.L.P. [Soares 2026a] SBSI 2026 (rejected).
      → prior work: arquitetura BTV
  [6] Soares, D.L.P. [Soares 2026b] ICLR 2026 AIWILD (rejected).
      → prior work: hybrid kernel

RECOMENDADAS (Related Work na Seção 6, pode aparecer na intro):
  [7] Open Policy Agent. rego.opa.io, 2024.
  [8] Burns et al. "Kubernetes: Scheduling the Future at Cloud Scale." 2016.
  [9] Henzinger et al. "Runtime Monitoring of Dynamic Fairness Properties." ACM, 2023.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ENTREGÁVEL ESPERADO
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Arquivo: section1_introduction.tex (LaTeX IEEE format)
  OU:     section1_introduction.md (aceito para revisão inicial)

Conteúdo:
  - 4 parágrafos conforme especificação acima
  - 500–650 palavras
  - Referências inline ([1], [2], etc.)
  - Sem subseções (IEEE Computer Magazine não usa subseções na intro)

Prazo interno sugerido: 28/março (deixa 16 dias para o resto)

Dúvidas sobre escopo ou contribuições: consultar este handoff.
Dúvidas sobre a prova do Teorema: ver github.com/danzeroum/silent-decisions-proof
Dúvidas sobre a implementação: ver github.com/danzeroum/BuildToValueGovernance

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
FIM DO HANDOFF
Versão: 1.0 | Data: 24 março 2026 | Sessão: BTV Paper P1 Destilação Final
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
