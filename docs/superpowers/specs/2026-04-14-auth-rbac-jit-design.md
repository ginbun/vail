# Vail Auth/RBAC/JIT Design

## Goal

Build the first secure authentication and authorization baseline for Vail:

- Password login + TOTP second factor
- JWT access token + one-time refresh token rotation
- RBAC permission checks on protected APIs
- JIT privilege elevation with approval and auto-expiry
- Weekly partitioned PostgreSQL 18 tables for high-volume security and session logs

## Scope

In scope (this phase):

- Backend: auth, MFA, token lifecycle, basic RBAC checks, JIT flow, audit and session metadata logging
- Frontend: login page with MFA step, token refresh integration, basic JIT request/approval UI hooks
- Database: schema additions for MFA, refresh token store, JIT workflow, weekly partitioned logs

Out of scope (next phases):

- Full enterprise SSO (OIDC/SAML)
- WebAuthn/Passkey
- Advanced risk-based adaptive auth

## Architecture

Single-service modular architecture in `vail-rs`:

- `auth`: username/password verification and login challenge issuance
- `mfa_totp`: TOTP enrollment and code verification
- `token`: JWT issuance, refresh rotation, revocation
- `rbac`: role-permission checks for request authorization
- `jit`: request/approve/revoke temporary privileges
- `audit`: append-only security event writes

Frontend (`vail-web`) consumes backend APIs only; all authorization decisions are backend-enforced.

## Data Model

### OLTP Tables

- `user_mfa_totp`
  - `user_id` (PK/FK), `secret_ciphertext`, `enabled`, `created_at`, `updated_at`
- `auth_refresh_token`
  - `id`, `user_id`, `token_hash`, `session_id`, `expires_at`, `rotated_at`, `revoked_at`, `created_at`
- `sys_permission`
  - `id`, `code`, `name`, `description`, `created_at`
- `sys_role_permission`
  - `role_id`, `permission_id`, `created_at`
- `jit_request`
  - `id`, `requester_id`, `reason`, `status`, `requested_at`, `expires_at`, `approved_at`, `approver_id`, `revoked_at`

### Weekly Partitioned Log Tables (PostgreSQL 18)

- `login_log` partitioned by week on `create_time`
- `operator_log` partitioned by week on `create_time`
- `ssh_session` partitioned by week on `start_time`

Partition management:

- Pre-create partitions for next 8-12 weeks
- Drop or detach expired partitions by retention policy
- Enforce time-bounded queries to maximize partition pruning

## API Flow

1. `POST /auth/login`
   - Validate password
   - Return `login_challenge_id` and `mfa_required=true` (no tokens yet)
2. `POST /auth/mfa/totp/verify`
   - Validate challenge + TOTP code
   - Issue access token + refresh token
3. `POST /auth/refresh`
   - Validate refresh token hash
   - Revoke old token, issue new access + refresh pair
4. `POST /auth/logout`
   - Revoke current refresh chain/session
5. `POST /jit/request`
   - Create JIT request with reason and desired duration
6. `POST /jit/approve`
   - Approver grants temporary elevation with expiry

## Security Controls

- No plaintext secrets in DB/logs/payloads
- Store only hashed refresh token server-side
- TOTP secret stored encrypted at rest (ciphertext only)
- Access token short TTL; refresh rotated on every use
- All auth/JIT/security events logged to immutable audit stream
- Source IP/request id/session id bound to key events

## Error Handling

- Uniform API error envelope with stable codes
- Authentication and authorization errors do not leak sensitive detail
- Refresh replay attempts are rejected and audited
- Expired or revoked JIT grants are denied by default

## Testing Strategy

- Unit tests
  - TOTP verify, refresh rotation, permission checks, JIT expiry checks
- Integration tests
  - Login -> MFA -> access, refresh rotation chain, JIT approval and expiration
- Security-focused tests
  - Refresh replay rejection
  - Missing permission and expired elevation denial
  - Sensitive field masking in responses/logs

## Rollout Plan

1. Add DB migration for auth/JIT tables and weekly partition routines
2. Refactor auth endpoints to challenge + MFA + rotation flow
3. Add RBAC/JIT guards and audit writes
4. Update frontend login to support TOTP step
5. Validate with integration tests and manual hardening checks

## Acceptance Criteria

- Login requires valid password and TOTP for enabled users
- Refresh token is one-time use and always rotated
- Protected APIs enforce RBAC and JIT expiration
- `login_log`, `operator_log`, and `ssh_session` run on weekly partitions
- Security audit events are written for auth, token, and JIT lifecycle
