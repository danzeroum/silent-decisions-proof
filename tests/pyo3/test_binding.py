"""
Teste 3 — PyO3 Binding Unit Tests

Valida que o binding Python respeita o perímetro do enclave Rust:
- Não aceita hash pré-computado (apenas bytes crus como input).
- Não expõe consume(), Clone, ou construção direta de SealedVerdict.
- Context manager garante teardown determinístico.
- Falhas do log sink propagam como BTVError.

Epistemic footer:
  Este teste valida que o binding PyO3 impede que o orquestrador Python
  forje um hash ou token. Ele NÃO garante não-repúdio ponta-a-ponta, pois
  isso depende de persistência durável e integridade de canal entre
  processos — componentes fora do escopo desta implementação de referência.
"""
import pytest
import btv_python
from btv_python import (
    issue_verdict,
    SealedVerdict,
    LogConfig,
    TestingLogConfig,
    BTVError,
)


# ─────────────────────────────────────────────────────────────────────────────
# Test A: valid verdict via context manager
# ─────────────────────────────────────────────────────────────────────────────

def test_valid_verdict_with_context_manager():
    """A verdict can be issued and used inside a `with` block."""
    cfg = TestingLogConfig()
    with issue_verdict(
        raw_context=b'{"applicant": "A", "score": 720}',
        decision="allow",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="Score above threshold",
        contestability_hours=720,
        log_config=cfg,
    ) as v:
        assert isinstance(v, SealedVerdict)
        assert v.verify_integrity() is True
        # Hash is 32 bytes BLAKE3 → 64 hex chars
        assert isinstance(v.hash_hex, str)
        assert len(v.hash_hex) == 64
        assert v.decision == "allow"
        assert v.jurisdiction == "BR-LGPD"
        assert v.appeal_deadline_hours == 720


def test_verdict_consumed_after_exit():
    """After `__exit__`, all accessors raise RuntimeError."""
    cfg = TestingLogConfig()
    v = issue_verdict(
        raw_context=b'{"score": 450}',
        decision="deny",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="Below threshold",
        contestability_hours=720,
        log_config=cfg,
    )
    with v:
        _ = v.hash_hex  # works inside
    with pytest.raises(RuntimeError, match="already consumed"):
        _ = v.hash_hex


# ─────────────────────────────────────────────────────────────────────────────
# Test B: forged hash attempt — Class F (THREAT MODEL)
# ─────────────────────────────────────────────────────────────────────────────

def test_forged_hash_attempt_rejected_str():
    """Python CANNOT pass a string as raw_context."""
    cfg = TestingLogConfig()
    with pytest.raises(TypeError, match="raw_context must be bytes"):
        issue_verdict(
            raw_context='{"forged": "string-not-bytes"}',  # WRONG type
            decision="allow",
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )


def test_forged_hash_attempt_rejected_dict():
    """Python CANNOT pass a dict as raw_context."""
    cfg = TestingLogConfig()
    with pytest.raises(TypeError, match="raw_context must be bytes"):
        issue_verdict(
            raw_context={"forged_hash": "abc123"},  # WRONG type
            decision="allow",
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )


def test_forged_hash_attempt_rejected_none():
    """Python CANNOT pass None as raw_context."""
    cfg = TestingLogConfig()
    with pytest.raises(TypeError, match="raw_context must be bytes"):
        issue_verdict(
            raw_context=None,
            decision="allow",
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )


def test_no_accept_prehashed_method():
    """The binding MUST NOT expose an `accept_precomputed_hash` API."""
    assert not hasattr(btv_python, "accept_precomputed_hash")
    assert not hasattr(btv_python, "from_hash")
    # SealedVerdict has no PUBLIC constructor that accepts a hash:
    # attempting to construct one with arguments must fail.
    with pytest.raises(TypeError):
        SealedVerdict(hash_hex="abc")  # type: ignore[call-arg]
    with pytest.raises(TypeError):
        SealedVerdict(b"\x00" * 32)  # type: ignore[call-arg]


# ─────────────────────────────────────────────────────────────────────────────
# Test C: invalid jurisdiction
# ─────────────────────────────────────────────────────────────────────────────

def test_unknown_jurisdiction_rejected():
    """ComplianceAuthority rejects unknown jurisdictions."""
    cfg = TestingLogConfig()
    with pytest.raises((ValueError, BTVError), match="jurisdiction"):
        issue_verdict(
            raw_context=b'ctx',
            decision="allow",
            jurisdiction="Narnia",  # not in allowlist
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )


# ─────────────────────────────────────────────────────────────────────────────
# Test D: invalid decision
# ─────────────────────────────────────────────────────────────────────────────

def test_invalid_decision_rejected():
    cfg = TestingLogConfig()
    with pytest.raises(ValueError, match="decision must be"):
        issue_verdict(
            raw_context=b'ctx',
            decision="maybe",  # invalid
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )


# ─────────────────────────────────────────────────────────────────────────────
# Test E: fail-secure when log unavailable (Test 5 connection)
# ─────────────────────────────────────────────────────────────────────────────

def test_fail_secure_when_log_unavailable():
    """When the log sink is unavailable, issue_verdict raises BTVError."""
    cfg = TestingLogConfig()
    cfg.fail()  # simulate partition
    with pytest.raises((RuntimeError, BTVError)) as exc_info:
        issue_verdict(
            raw_context=b'{"score": 800}',
            decision="allow",
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )
    # Error message must mention "unavailable" or "log" (fail-secure)
    msg = str(exc_info.value).lower()
    assert "unavailable" in msg or "log" in msg, f"unexpected error: {exc_info.value}"
    # No record should have been appended
    assert cfg.is_empty()


def test_fail_secure_when_append_fails_after_recovery():
    """After recovery, issue_verdict works again."""
    cfg = TestingLogConfig()
    cfg.fail()
    with pytest.raises((RuntimeError, BTVError)):
        issue_verdict(
            raw_context=b'ctx',
            decision="allow",
            jurisdiction="BR-LGPD",
            policy_version="1.0.0",
            explanation="x",
            contestability_hours=720,
            log_config=cfg,
        )
    cfg.recover()
    with issue_verdict(
        raw_context=b'ctx',
        decision="allow",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="x",
        contestability_hours=720,
        log_config=cfg,
    ) as v:
        assert v.verify_integrity()


# ─────────────────────────────────────────────────────────────────────────────
# Test F: SQLite-backed log (durable)
# ─────────────────────────────────────────────────────────────────────────────

def test_sqlite_log_backend(tmp_path):
    """SQLite backend persists verdicts durably."""
    db_path = str(tmp_path / "btv.db")
    cfg = LogConfig.sqlite(db_path)
    with issue_verdict(
        raw_context=b'durable-context',
        decision="deny",
        jurisdiction="EU-GDPR",
        policy_version="2024/1689",
        explanation="GDPR Art. 22 denial",
        contestability_hours=720,
        log_config=cfg,
    ) as v:
        assert v.jurisdiction == "EU-GDPR"
        assert v.verify_integrity()


# ─────────────────────────────────────────────────────────────────────────────
# Test G: idempotency of evidence_id (deterministic BLAKE3)
# ─────────────────────────────────────────────────────────────────────────────

def test_evidence_id_is_deterministic():
    """Same raw_context produces same evidence_id hash."""
    cfg1 = TestingLogConfig()
    cfg2 = TestingLogConfig()
    with issue_verdict(
        raw_context=b'same-context',
        decision="allow",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="x",
        contestability_hours=720,
        log_config=cfg1,
    ) as v1:
        h1 = v1.hash_hex
    with issue_verdict(
        raw_context=b'same-context',
        decision="allow",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="x",
        contestability_hours=720,
        log_config=cfg2,
    ) as v2:
        h2 = v2.hash_hex
    assert h1 == h2


# ─────────────────────────────────────────────────────────────────────────────
# Test H: SealedVerdict is read-only
# ─────────────────────────────────────────────────────────────────────────────

def test_sealed_verdict_is_read_only():
    """SealedVerdict attributes are read-only (no setters)."""
    cfg = TestingLogConfig()
    with issue_verdict(
        raw_context=b'ctx',
        decision="allow",
        jurisdiction="BR-LGPD",
        policy_version="1.0.0",
        explanation="original",
        contestability_hours=720,
        log_config=cfg,
    ) as v:
        with pytest.raises(AttributeError):
            v.decision = "deny"  # type: ignore[misc]
        with pytest.raises(AttributeError):
            v.explanation = "tampered"  # type: ignore[misc]
