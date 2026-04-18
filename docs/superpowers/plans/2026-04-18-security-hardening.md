# Dependency Security Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the remaining 7 vulnerabilities in `vail-web` by upgrading top-level dev dependencies and enforcing safe versions of sub-dependencies via `overrides`.

**Architecture:** Use NPM `overrides` to patch deep-seated vulnerabilities in the supply chain without waiting for upstream package updates.

**Tech Stack:** NPM, Vite 8, ESLint 9.

---

### Task 1: Upgrade Dev Dependencies and Add Security Overrides

**Files:**
- Modify: `vail-web/package.json`

- [ ] **Step 1: Update `package.json` with new versions and overrides**

Modify `vail-web/package.json` to:
1. Upgrade `lint-staged` to `^16.4.0`.
2. Upgrade `@commitlint/cli` and `@commitlint/config-conventional` to `^19.7.1`.
3. Add an `overrides` block to enforce safe versions.

```json
{
  "devDependencies": {
    "@commitlint/cli": "^19.7.1",
    "@commitlint/config-conventional": "^19.7.1",
    "lint-staged": "^16.4.0",
    ...
  },
  "overrides": {
    "serialize-javascript": "^7.0.5",
    "yaml": "^2.8.3",
    "micromatch": "^4.0.8"
  }
}
```

- [ ] **Step 2: Run `npm install` to apply changes**

Run: `npm install --legacy-peer-deps` (required for Vite 8 compatibility with some plugins).

- [ ] **Step 3: Verify with `npm audit`**

Run: `npm audit`
Expected: 0 vulnerabilities.

- [ ] **Step 4: Verify build and lint**

Run: `npm run build && npm run type:check`
Expected: SUCCESS.

- [ ] **Step 5: Commit changes**

```bash
git add package.json package-lock.json
git commit -m "security: upgrade lint-staged and add dependency overrides to fix vulnerabilities"
```
