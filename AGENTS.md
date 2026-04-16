# Vail Multi-Agent Development Protocol

## Agent Roles

- Backend Agent (Primary): `GPT-5.3 Codex`
  - Scope: `vail-rs/` (API, auth, SSH/SFTP session proxy, DB schema, migrations, middleware)
  - Ownership: security-critical logic, protocol correctness, performance-sensitive paths
- Frontend Agent (Primary): `Gemini 3 Flash Preview`
  - Scope: `vail-web/` (Svelte pages/components, API integration, UX for bastion workflows)
  - Ownership: secure interaction flows, operator usability, audit visualization
- Security Gate Agent (Cross-cutting): reviewer role (can be either model)
  - Scope: threat modeling, security checklist validation, release gate decisions
  - Ownership: blocks merge if any critical control is missing

## Collaboration Contract

- Backend and frontend work in parallel but share one contract source: OpenAPI + auth/audit event schema.
- Any API touching auth, credentials, session control, or file transfer requires a short threat note in PR.
- Frontend cannot bypass backend authorization; UI permissions are display hints only.
- Security Gate Agent signs off every feature PR before merge.
- Prefer PostgreSQL 18 native partitioned tables for large time-series and audit data to simplify operations and reduce application-side complexity.

## Bastion Project Security Rules (Mandatory)

### 1) Identity and Access Control

- Enforce least privilege with RBAC by default; use ABAC only for explicit host/session constraints.
- Require MFA for all privileged accounts.
- Use short-lived tokens and rotate refresh tokens on every renewal.
- Support just-in-time privilege elevation with approval and auto-expiry.

### 2) Credential and Key Management

- Never store plaintext passwords, private keys, or tokens in DB/logs/cache.
- Encrypt secrets at rest with envelope encryption (KMS/HSM-managed DEK wrapping).
- Keep runtime secrets in dedicated secret manager; do not commit any secret into git.
- Rotate host credentials and application keys regularly; define emergency rotation runbook.

### 3) Session and Command Security

- All SSH/RDP/SFTP traffic must go through bastion proxy; no direct target host access from client.
- Bind every session to user identity, source IP/device context, and authorization decision.
- Record session metadata and command stream with tamper-evident storage.
- Provide high-risk command controls (deny/block/approval) for production assets.

### 4) Audit and Compliance

- Audit logs must be append-only, time-synchronized, and immutable for retention period.
- Log who did what, where, when, and why (ticket/request id) for each privileged operation.
- Protect logs with integrity checks (hash chain or signed batches).
- Separate audit-read privilege from system-admin privilege.

### 5) Data and Transport Protection

- Enforce TLS 1.2+ everywhere; prefer mTLS for service-to-service traffic.
- Encrypt sensitive columns and backups; verify restore procedures periodically.
- Mask sensitive fields in UI and logs by default.
- Define strict data retention and deletion policies for session recordings and transfer artifacts.

### 6) Secure Development Lifecycle

- Every feature starts with abuse-case review (session hijack, privilege escalation, data exfiltration).
- Required checks in CI: SAST, dependency/license scan, secret scan, unit/integration tests.
- High-risk modules (auth, crypto, session proxy, file transfer) require security-focused tests.
- Do not merge if critical vulnerabilities are open without documented risk acceptance.

### 7) Supply Chain and Runtime Hardening

- Pin dependency versions and verify signatures/checksums when possible.
- Use minimal base images, run as non-root, and drop unnecessary Linux capabilities.
- Apply security headers and strict CORS/CSRF policies for web endpoints.
- Restrict egress network paths from bastion services to required destinations only.

## Branch and Repository Guardrails

- `orion-visor/` is reference-only and must not be committed to this repository.
- Keep implementation changes in `vail-rs/` and `vail-web/` unless infra/docs update is required.
- Before commit, verify staged files exclude `orion-visor/`.

## Definition of Done (Security)

- Threat note updated.
- Security checks pass in CI.
- Audit fields/events validated.
- No plaintext secret exposure in code, logs, DB migrations, or frontend payloads.
- Security Gate Agent approval completed.
