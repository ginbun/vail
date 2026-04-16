# Vail Hosts & Permissions Visibility Design

## Goal

Deliver a usable post-login experience where users can clearly see:

- Which hosts they are authorized to access
- Which roles/permissions they currently hold

The first iteration must be desktop-table first while staying mobile-friendly for login and daily operations.

## Scope

In scope:

- Backend self-scoped IAM read APIs for current user
- Frontend split views: `Hosts` and `Permissions`
- Responsive UX for desktop and mobile
- Consistent loading, empty, and error states

Out of scope:

- JIT elevation workflow UI and control gates (kept in README TODO)
- Full admin console for user/role management
- Advanced host filtering/pagination/search APIs

## Information Architecture

Post-login navigation contains two primary pages:

1. `Hosts`
   - Shows host resources visible to the current user
2. `Permissions`
   - Shows role list, permission list, and resource summary for the current user

Navigation behavior:

- Desktop: top tab navigation
- Mobile: compact bottom navigation or stacked tab bar with large touch targets
- Optional hash state (`#hosts`, `#permissions`) to preserve active view on refresh

## Backend API Contract

### `GET /api/iam/me/summary`

Purpose: data source for the permissions page summary and lists.

Response shape:

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "user_id": 1,
    "roles": ["admin"],
    "permissions": ["iam.user-permission.view", "iam.user-resource.assign"],
    "host_access_count": 12
  }
}
```

### `GET /api/iam/me/hosts`

Purpose: data source for hosts visibility page.

Behavior:

- Use authenticated JWT user identity only
- Join `user_host_access` with `host`
- Filter soft-deleted resources (`host.deleted = 0`)
- Return host list compatible with existing frontend host table model

## Authorization & Security Rules

- Both `/iam/me/*` APIs require a valid login token
- APIs must not accept arbitrary user id parameters
- Response must never include sensitive credential material (`credential_data`)
- Admin-only assignment APIs remain separate (`/iam/users/:id/*`)
- Access events should be auditable (at minimum by standard API log pipeline)

## Frontend UX Design

### Hosts View

- Desktop-first table columns: `Name`, `Host`, `Port`, `Username`, `Status`
- Local keyword filter (name/hostname)
- Empty state: clear message and guidance text
- Error state: retry action and short human-readable hint

Mobile adaptation:

- Replace wide table with stacked host cards
- Preserve core fields and status badge
- Prevent horizontal scrolling as primary interaction mode

### Permissions View

- Summary cards:
  - Role count
  - Permission count
  - Host access count
- Detailed sections:
  - Role list
  - Permission list
- Mobile: collapse sections with compact spacing and touch-friendly controls

## UI Consistency Rules

- Reuse existing design tokens in `app.css` (`--primary`, `--bg`, `--border`, etc.)
- Keep state handling consistent across views:
  - loading
  - empty
  - error
- Minimum interactive target size for mobile controls: 40px+

## Testing & Verification

Backend:

- `cargo check`
- Add/adjust unit test for identity extraction and self-scoped query behavior where practical

Frontend:

- `npm run build`
- Manual viewport checks for desktop and mobile breakpoints

Functional acceptance:

- User can log in and see `Hosts` + `Permissions` views
- `Hosts` reflects assigned resources only
- `Permissions` reflects role/permission/resource summary for current user
- Mobile layout is usable without horizontal table dependence

## Rollout Notes

1. Implement `/api/iam/me/summary` and `/api/iam/me/hosts`
2. Add frontend split views and navigation state
3. Integrate responsive card/table behavior
4. Validate with build and manual interaction checks
5. Keep JIT as later-phase TODO
