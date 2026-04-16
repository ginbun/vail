# Hosts Permissions Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement user-facing Hosts and Permissions pages so authenticated users can view their authorized hosts and effective role/permission summary.

**Architecture:** Add two self-scoped backend IAM endpoints (`/api/iam/me/summary`, `/api/iam/me/hosts`) and consume them in a split Dashboard UI with desktop table-first layout and mobile card adaptation. Keep authorization decisions server-side by deriving user identity from JWT only.

**Tech Stack:** Rust (Axum, SQLx), Svelte 5, Axios, existing CSS variable system.

---

### Task 1: Backend self-scoped IAM endpoints

**Files:**
- Modify: `vail-rs/src/api/iam.rs`
- Modify: `vail-rs/src/model/mod.rs`

- [ ] **Step 1: Write failing tests for helper logic**

```rust
#[test]
fn normalize_ids_removes_duplicates_and_invalid_values() {
    let ids = vec![5, 2, 5, 0, -1, 2, 9];
    assert_eq!(normalize_ids(ids), vec![2, 5, 9]);
}
```

- [ ] **Step 2: Run test to verify baseline**

Run: `cargo test iam::tests::normalize_ids_removes_duplicates_and_invalid_values`
Expected: PASS (existing baseline stays green)

- [ ] **Step 3: Add response models for self endpoints**

```rust
pub struct IamMeSummaryResponse {
    pub user_id: i64,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub host_access_count: i64,
}
```

- [ ] **Step 4: Implement `GET /iam/me/summary`**

```rust
.route("/iam/me/summary", get(get_me_summary))
```

Use `guard::current_user_id(...)` and query role codes, permission codes, and count from `user_host_access`.

- [ ] **Step 5: Implement `GET /iam/me/hosts`**

```rust
.route("/iam/me/hosts", get(get_me_hosts))
```

Join `user_host_access` and `host`, return only `deleted = 0` rows with host fields used by frontend.

- [ ] **Step 6: Run backend compile verification**

Run: `cargo check`
Expected: PASS

### Task 2: Frontend API contracts for self-scoped visibility

**Files:**
- Modify: `vail-web/src/lib/api.ts`

- [ ] **Step 1: Add type definitions for IAM self endpoints**

```ts
export interface IamMeSummary {
  user_id: number;
  roles: string[];
  permissions: string[];
  host_access_count: number;
}
```

- [ ] **Step 2: Add IAM API calls**

```ts
export const iamApi = {
  meSummary: () => api.get<ApiResponse<IamMeSummary>>('/iam/me/summary'),
  meHosts: () => api.get<ApiResponse<Host[]>>('/iam/me/hosts')
};
```

- [ ] **Step 3: Run frontend build check**

Run: `npm run build`
Expected: PASS

### Task 3: Split Dashboard into Hosts and Permissions views

**Files:**
- Modify: `vail-web/src/routes/Dashboard.svelte`
- Optionally Create: `vail-web/src/routes/dashboard/HostsView.svelte`
- Optionally Create: `vail-web/src/routes/dashboard/PermissionsView.svelte`

- [ ] **Step 1: Add view state for two pages**

```ts
let activeView = $state<'hosts' | 'permissions'>('hosts');
```

- [ ] **Step 2: Replace task-centric UI with hosts + permissions**

Render:
- `Hosts` tab: authorized host table
- `Permissions` tab: summary cards + role/permission lists

- [ ] **Step 3: Integrate `iamApi.meSummary()` and `iamApi.meHosts()`**

Load both in `onMount`, map to local state, and provide loading/error/empty UX.

- [ ] **Step 4: Keep desktop table-first and add mobile adaptation**

Add responsive CSS:
- desktop shows table layout
- mobile (`max-width: 768px`) switches host rows to stacked cards and keeps tap targets >= 40px

- [ ] **Step 5: Run frontend build check**

Run: `npm run build`
Expected: PASS

### Task 4: End-to-end verification and documentation sync

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README current focus wording**

Document that first visible feature set is:
- My Hosts visibility
- My Permissions visibility

- [ ] **Step 2: Run backend checks**

Run: `cargo check && cargo test iam::tests::normalize_ids_removes_duplicates_and_invalid_values`
Expected: PASS

- [ ] **Step 3: Run frontend checks**

Run: `npm ci && npm run build`
Expected: PASS

- [ ] **Step 4: Optional compose validation**

Run: `docker compose config`
Expected: valid compose output without errors
