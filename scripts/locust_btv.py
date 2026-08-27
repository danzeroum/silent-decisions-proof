"""
Locust load test script for BTV HTTP deployment.

USAGE (requires running an HTTP server that wraps `btv_python.issue_verdict`):

    locust -f scripts/locust_btv.py --host http://localhost:8080

The server must expose POST /verdict accepting JSON:
    {"raw_context_b64": "...", "decision": "allow|deny",
     "jurisdiction": "BR-LGPD", "policy_version": "1.0.0",
     "explanation": "...", "contestability_hours": 720}

This script is provided for the manuscript's reproduction instructions.
It is NOT executed by the artifact-v2 test pipeline because the pipeline
does not run an HTTP server; for in-process concurrency benchmarks see
`btv-core/tests/test_load.rs` instead.

Epistemic footer:
    Este script valida a sobrevivência do BTV sob carga HTTP realista.
    Ele NÃO é executado no pipeline do artifact-v2 (que usa benchmark
    in-process em Rust via rayon). Para reprodução, iniciar um servidor
    HTTP que envolva `btv_python.issue_verdict` e executar este script
    contra ele.
"""
import base64
import json
from locust import HttpUser, task, between


class BTVUser(HttpUser):
    wait_time = between(0.01, 0.1)

    @task
    def issue_verdict(self):
        ctx = json.dumps({"applicant": "anon", "score": 700}).encode("utf-8")
        payload = {
            "raw_context_b64": base64.b64encode(ctx).decode("ascii"),
            "decision": "allow",
            "jurisdiction": "BR-LGPD",
            "policy_version": "1.0.0",
            "explanation": "locust load test",
            "contestability_hours": 720,
        }
        with self.client.post("/verdict", json=payload, catch_response=True) as resp:
            if resp.status_code == 200:
                resp.success()
            elif resp.status_code == 503:
                # Fail-secure: log unavailable. Count as expected failure.
                resp.failure("log unavailable (fail-secure)")
            else:
                resp.failure(f"unexpected status: {resp.status_code}")
