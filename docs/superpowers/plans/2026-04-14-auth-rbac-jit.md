# Auth RBAC JIT Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver secure auth baseline with TOTP challenge flow, refresh token rotation, JIT elevation, and PostgreSQL 18 weekly-partitioned security/session logs.

**Architecture:** Keep `vail-rs` as a modular single service. Introduce auth challenge + TOTP verification endpoints, rotate refresh tokens server-side, and add migration-driven weekly partitions for high-volume tables. Update `vail-web` login flow to support MFA challenge step and refresh handling.

**Tech Stack:** Rust (Axum, SQLx, JWT), PostgreSQL 18 partitioned tables, Svelte 5 + Axios.

---

### Task 1: Database foundation for auth/JIT + weekly partitions

**Files:**
- Create: `vail-rs/migrations/V4__auth_jit_weekly_partitions.sql`
- Modify: `vail-rs/src/db/migrate.rs`

- [ ] Add tables for `user_mfa_totp`, `auth_login_challenge`, `auth_refresh_token`, `sys_permission`, `sys_role_permission`, `jit_request`.
- [ ] Rebuild `login_log`, `operator_log`, and `ssh_session` as weekly partitioned tables.
- [ ] Add helper SQL functions to pre-create weekly partitions.
- [ ] Update startup partition warm-up calls in `vail-rs/src/db/migrate.rs`.

### Task 2: Backend auth + MFA + refresh rotation

**Files:**
- Modify: `vail-rs/src/model/mod.rs`
- Modify: `vail-rs/src/api/auth.rs`
- Modify: `vail-rs/Cargo.toml`

- [ ] Add DTOs for login challenge and TOTP verify requests/responses.
- [ ] Refactor `POST /auth/login` to return challenge when MFA enabled.
- [ ] Add `POST /auth/mfa/totp/verify` to issue tokens after TOTP validation.
- [ ] Rotate refresh token on every `POST /auth/refresh` call.
- [ ] Revoke refresh token on `POST /auth/logout`.

### Task 3: Frontend MFA login + refresh payload updates

**Files:**
- Modify: `vail-web/src/lib/api.ts`
- Modify: `vail-web/src/lib/auth.ts`
- Modify: `vail-web/src/routes/Login.svelte`

- [ ] Add API contracts for challenge/TOTP verify response shapes.
- [ ] Update login screen to two-step flow (password then TOTP code).
- [ ] Ensure refresh token updates after refresh rotation response.

### Task 4: Verification

**Files:**
- Modify if needed: touched files above

- [ ] Run `cargo check` in `vail-rs` and fix compile errors.
- [ ] Run `npm run build` in `vail-web` and fix type/build errors.
- [ ] Perform quick endpoint sanity review for auth and MFA flow.
