# Vail-Web 前端依赖漏洞修复与优化方案

> 基于 `vail-web/package.json` (git HEAD: commit `a3bd7d9`) 的全面安全审计

## 审计概览

当前 `vail-web/package.json` 直接复制自 Orion Visor UI v2.5.7，存在 **多个已知 CVE 漏洞、EOL 依赖、密码学缺陷和供应链风险**。作为堡垒机前端，安全标准必须高于普通 Web 应用。

| 风险等级 | 数量 | 说明 |
|---------|------|------|
| 🔴 严重 (Critical) | 4 | 已有公开 CVE，可被远程利用 |
| 🟠 高危 (High) | 3 | 密码学缺陷 / 已废弃且有已知漏洞 |
| 🟡 中危 (Medium) | 5 | EOL / 无维护 / 供应链风险 |
| 🔵 优化 (Optimization) | 6 | 包体积优化 / 现代化替换 |

---

## User Review Required

> [!CAUTION]
> **Vite 3.x → 8.x 大版本跨越**：Vite 3 → 8 跨越 5 个主版本，配置文件 (`vite.config.*.ts`)、插件 API 和 `index.html` 入口均有重大变更。需要全量回归测试。

> [!IMPORTANT]
> **`jsencrypt` (RSA 前端加密) 的去留**：Orion 使用 `jsencrypt` 在前端做 RSA 加密后传输密码。由于 Vail 后端已强制 TLS 1.2+，前端 RSA 加密是否保留需要明确决策。建议移除，改为 TLS + Argon2 后端 hash。

> [!WARNING]
> **`@dangojs/*` 和 `@sanqi377/*` 供应链风险**：这些是低下载量、不知名维护者的 npm 包，在堡垒机项目中引入未审计的第三方 UI 包是高风险行为。

---

## Open Questions

1. **Vite 升级策略**：直接升级到 Vite 8.x（最新稳定）还是分步升级到 Vite 6.x 再到 8.x？考虑到项目尚未正式上线，建议一步到位。
2. **`jsencrypt` 去留**：前端 RSA 加密是否保留？后端已使用 Argon2 + TLS，前端 RSA 层是否多余？
3. **`@dangojs/*` / `@sanqi377/*` 处理方式**：是 fork 审计后内部维护，还是直接用 Arco Design 原生组件替代？
4. **`html2canvas` 用途**：此包主要用于截屏导出，是否为 MVP 必需功能？如非必需建议延后引入。
5. **`echarts` 用途**：仪表盘图表是否在 Phase 1 范围内？如不在，可延后引入以减小包体积。

---

## 一、🔴 严重漏洞修复 (Critical CVE Fixes)

### 1.1 Vite 3.2.5 → 8.x（多个 CVE）

| 字段 | 详情 |
|------|------|
| **当前版本** | `"vite": "^3.2.5"` |
| **目标版本** | `"vite": "^8.0.8"` |
| **影响 CVE** | 多个任意文件读取 / 路径穿越漏洞 (dev server `?raw`/`?import` 参数绕过 `server.fs.deny`) |
| **攻击面** | 使用 `--host` 暴露 dev server 时，攻击者可读取 `.env`、源码、系统配置文件 |
| **堡垒机影响** | **极高** — 开发环境可能包含数据库凭证、SSH 密钥路径等敏感配置 |

**迁移工作**：
- 更新 `@vitejs/plugin-vue` → `^5.x`
- 更新 `@vitejs/plugin-vue-jsx` → `^4.x`
- 重写 `config/vite.config.base.ts`、`config/vite.config.dev.ts`、`config/vite.config.prod.ts`
- 移除不兼容插件（`vite-plugin-eslint`、`vite-plugin-imagemin`、`vite-plugin-compression`）
- 更新 `index.html` 入口（Vite 8 要求项目根目录）

---

### 1.2 Rollup 3.9.1 → 4.x（CVE-2024-47068）

| 字段 | 详情 |
|------|------|
| **当前版本** | `"rollup": "^3.9.1"` + `resolutions` 锁定 `"rollup": "^2.56.3"` |
| **目标版本** | 移除显式依赖，由 Vite 8 自动管理 (Rollup 4.x) |
| **CVE** | CVE-2024-47068 — DOM Clobbering → XSS |
| **攻击面** | `import.meta.url` 在 `cjs`/`umd`/`iife` 格式输出时可被注入恶意 HTML 执行任意 JS |
| **堡垒机影响** | **高** — 如果终端组件或文件管理器使用了 `import.meta`，可导致 XSS 进而窃取会话 |

**迁移工作**：
- 删除 `devDependencies` 中的 `"rollup"` 显式声明
- 删除 `resolutions` 中的 `"rollup": "^2.56.3"`（这个 resolution 反而把版本压低到更危险的 2.x）
- Vite 8 内置 Rollup 4.x，无需手动管理

---

### 1.3 Axios 1.7.9（CVE-2025-27152 + CVE-2025-58754 + CVE-2026-25639）

| 字段 | 详情 |
|------|------|
| **当前版本** | `"axios": "^1.7.9"` |
| **目标版本** | `"axios": "^1.9.0"` (或最新稳定) |
| **CVE** | CVE-2025-27152 (SSRF/凭证泄露)、CVE-2025-58754 (DoS)、CVE-2026-25639 (DoS) |
| **攻击面** | SSRF 可泄露 `X-API-KEY` 等认证头到第三方；DoS 可通过 `data:` URL 或 `__proto__` 消耗内存 |
| **堡垒机影响** | **极高** — 前端 API 层直接与认证系统交互，SSRF 可泄露 JWT Token |

**迁移工作**：
- 直接升级版本号，API 无破坏性变更
- 审查所有 `axios.create()` 实例确保 `baseURL` 配置正确

---

### 1.4 mockjs（CVE-2023-26158 原型污染）

| 字段 | 详情 |
|------|------|
| **当前版本** | `"mockjs": "^1.1.0"` |
| **目标版本** | **移除**，替换为 MSW (Mock Service Worker) 或直接删除 |
| **CVE** | CVE-2023-26158 — Prototype Pollution |
| **状态** | 项目已停止维护，不会有安全修复 |

**迁移工作**：
- 从 `devDependencies` 中移除 `mockjs` 和 `@types/mockjs`
- 如需 mock 能力，引入 `msw@^2.x`

---

## 二、🟠 高危问题 (High Severity)

### 2.1 `jsencrypt` — 前端 RSA 加密设计缺陷

| 字段 | 详情 |
|------|------|
| **当前版本** | `"jsencrypt": "^3.3.2"` |
| **建议** | **移除** |
| **问题** | Orion 使用前端 RSA 加密密码后传输给后端解密。在已有 TLS 的情况下这是安全反模式 — RSA 密钥必须通过接口下发，引入额外攻击面 |

> [!WARNING]
> 堡垒机规则要求"所有 SSH/RDP/SFTP 流量必须通过堡垒机代理"，前端不应承担任何加密职责。密码应通过 TLS 直接传输，后端使用 Argon2 哈希。

---

### 2.2 `ts-md5` — MD5 密码学已破解

| 字段 | 详情 |
|------|------|
| **当前版本** | `"ts-md5": "^1.3.1"` |
| **建议** | **移除**（如仅用于非安全场景如文件校验和，替换为 Web Crypto API `SHA-256`）|
| **问题** | MD5 碰撞攻击已实用化，不应在任何安全上下文中使用 |

**替换方案**：
```typescript
// 文件校验和场景，使用浏览器原生 Web Crypto API
async function sha256(data: ArrayBuffer): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', data);
  return Array.from(new Uint8Array(hash))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}
```

---

### 2.3 `html2canvas` — 停止维护 + XSS 风险

| 字段 | 详情 |
|------|------|
| **当前版本** | `"html2canvas": "^1.4.1"` |
| **建议** | 移除或替换为 `html-to-image`（如功能必需）|
| **问题** | 项目几乎停止维护；DOM 截图库天然面临 XSS 注入风险 |

---

## 三、🟡 中危 — EOL / 供应链风险

### 3.1 ESLint 8.x + typescript-eslint v5 — 已 EOL

| 包名 | 当前版本 | 目标版本 |
|------|----------|----------|
| `eslint` | `^8.56.0` | `^9.x` (Flat Config) |
| `@typescript-eslint/eslint-plugin` | `^5.40.0` | `^8.x` |
| `@typescript-eslint/parser` | `^5.40.0` | `^8.x` |
| `eslint-config-airbnb-base` | `^15.0.0` | 移除，改用 `@eslint/js` |
| `eslint-config-prettier` | `^8.5.0` | `^10.x` |
| `eslint-plugin-prettier` | `^4.2.1` | `^5.x` |
| `eslint-plugin-import` | `^2.26.0` | `eslint-plugin-import-x` |
| `eslint-import-resolver-typescript` | `^3.5.1` | 随 `eslint-plugin-import-x` 更新 |

**说明**：ESLint 8 于 2024-10-05 EOL，不再接收安全补丁。typescript-eslint v5 更早废弃。

**迁移工作**：
- 将 `.eslintrc.js` 迁移为 `eslint.config.js` (Flat Config)
- 移除 `eslint-config-airbnb-base`，用 `@eslint/js` recommended 替代
- 统一更新所有 ESLint 相关插件

---

### 3.2 TypeScript 4.8.4 — 严重落后

| 字段 | 详情 |
|------|------|
| **当前版本** | `"typescript": "^4.8.4"` |
| **目标版本** | `"typescript": "^5.7.0"` |
| **问题** | TS 4.x 缺少 `satisfies`、`const` 泛型参数、装饰器等关键特性；与现代 Vue 3 工具链不兼容 |

---

### 3.3 `@dangojs/*` / `@sanqi377/*` — 供应链风险

| 包名 | 风险评估 |
|------|----------|
| `@dangojs/a-query-header` | 低下载量 npm 包，未知维护者，无安全审计 |
| `@dangojs/digitforce-ui-utils` | 同上 |
| `@sanqi377/arco-vue-icon-picker` | 同上 |

> [!CAUTION]
> 堡垒机安全规则要求"Pin dependency versions and verify signatures/checksums when possible"。这些包不满足供应链安全标准。

**建议**：
- 审计源码后 fork 到内部 registry，或用 Arco Design 原生组件替代
- 如功能简单（查询头、图标选择器），建议自行实现

---

### 3.4 `vue-tsc` 1.x → 已废弃

| 字段 | 详情 |
|------|------|
| **当前版本** | `"vue-tsc": "^1.0.14"` |
| **目标版本** | `"vue-tsc": "^2.x"` |
| **问题** | vue-tsc 1.x 基于 Volar 1.x，不兼容 TS 5.x 和 Vue 3.5+ 的新特性 |

---

### 3.5 `resolutions` 字段异常

```json
"resolutions": {
  "bin-wrapper": "npm:bin-wrapper-china",  // 供应链风险：替换为中国镜像包
  "rollup": "^2.56.3",                     // 将 rollup 降级到有漏洞的 2.x
  "gifsicle": "5.2.0"                      // 锁定 gifsicle 版本
}
```

> [!WARNING]
> `bin-wrapper-china` 是非官方 npm 包，将标准依赖重定向到第三方维护的版本，这是典型的供应链攻击面。必须移除。

---

## 四、🔵 优化建议 (Optimization)

### 4.1 包体积优化

| 操作 | 当前 | 建议 | 预估节省 |
|------|------|------|----------|
| `lodash` → `lodash-es` + tree-shake | `lodash` 全量引入 ~70KB gzipped | 按需导入 `import { debounce } from 'lodash-es'` | ~50KB |
| `echarts` 按需引入 | 全量 ~800KB | 使用 `echarts/core` + 注册器 | ~500KB |
| `monaco-editor` 按需加载 | ~5MB | 使用 `monaco-editor/esm/vs/editor/editor.api` + worker | ~3MB |
| `cron-parser` 评估必要性 | 是否 Phase 1 必需？ | 延后引入 | ~15KB |

### 4.2 依赖精简

以下包可在不影响功能的前提下移除/替换：

| 包名 | 理由 | 替换方案 |
|------|------|----------|
| `file-saver` | 浏览器原生 `Blob` + `URL.createObjectURL` 已足够 | 自行封装 3 行代码 |
| `nprogress` | 可用 CSS 动画 + Arco 进度条组件替代 | Arco `<a-progress>` |
| `query-string` | 浏览器原生 `URLSearchParams` 已覆盖大部分场景 | `URLSearchParams` |
| `mitt` | Vue 3 provide/inject + Pinia 已覆盖事件总线场景 | Pinia / composables |
| `sortablejs` | 评估是否 MVP 必需 | 延后引入 |

### 4.3 Stylelint 更新

| 包名 | 当前 | 目标 |
|------|------|------|
| `stylelint` | `^16.12.0` | ✅ 已是较新版本 |
| `stylelint-config-prettier` | `^9.0.5` | **移除** — Stylelint 16 已内置 Prettier 兼容 |
| `stylelint-config-rational-order` | `^0.1.2` | 替换为 `stylelint-config-clean-order` |

### 4.4 Prettier 升级

| 字段 | 详情 |
|------|------|
| **当前版本** | `"prettier": "^2.7.1"` |
| **目标版本** | `"prettier": "^3.x"` |
| **说明** | Prettier 3 改为 ESM，需更新配置文件格式 |

---

## 五、Node.js 引擎要求更新

```diff
  "engines": {
-   "node": ">=14.0.0"
+   "node": ">=20.0.0"
  }
```

**理由**：Node.js 14 和 16 均已 EOL。Vite 8 最低要求 Node 20。CI 已配置 `node-version: 20`。

---

## 六、Proposed Changes — 最终 package.json 结构

### [MODIFY] [package.json](file:///home/baize/Projects/vail/vail-web/package.json)

目标 `package.json` 核心变更摘要：

```diff
  {
-   "name": "orion-visor-ui",
+   "name": "vail-web",
-   "description": "Orion Visor UI",
+   "description": "Vail Bastion Host Web UI",
-   "version": "2.5.7",
+   "version": "0.1.0",

    "dependencies": {
      "@arco-design/web-vue": "^2.56.3",    // ✅ 保留
-     "@dangojs/a-query-header": ...,        // ❌ 移除：供应链风险
-     "@dangojs/digitforce-ui-utils": ...,   // ❌ 移除：供应链风险
-     "@sanqi377/arco-vue-icon-picker": ..., // ❌ 移除：供应链风险
      "@vueuse/core": "^12.3.0",            // ✅ 保留
      "@xterm/*": "latest stable",           // ✅ 保留，升级到最新
-     "axios": "^1.7.9",
+     "axios": "^1.9.0",                     // 🔴 CVE 修复
-     "html2canvas": "^1.4.1",               // ❌ 移除
-     "jsencrypt": "^3.3.2",                 // ❌ 移除：安全反模式
-     "lodash": "^4.17.21",
+     "lodash-es": "^4.17.21",              // 🔵 tree-shake 优化
-     "ts-md5": "^1.3.1",                    // ❌ 移除：MD5 已破解
-     "file-saver": "^2.0.5",               // ❌ 移除：浏览器原生替代
-     "nprogress": "^0.2.0",                // ❌ 移除：Arco 组件替代
-     "query-string": "^9.1.1",             // ❌ 移除：URLSearchParams 替代
-     "mitt": "^3.0.0",                      // ❌ 移除：Pinia 替代
    },

    "devDependencies": {
-     "vite": "^3.2.5",
+     "vite": "^8.0.8",                      // 🔴 CVE 修复
-     "rollup": "^3.9.1",                    // ❌ 移除：Vite 内置管理
-     "@vitejs/plugin-vue": "^3.1.2",
+     "@vitejs/plugin-vue": "^5.x",
-     "typescript": "^4.8.4",
+     "typescript": "^5.7.0",
-     "eslint": "^8.56.0",
+     "eslint": "^9.x",                      // Flat Config
-     "mockjs": "^1.1.0",                    // ❌ 移除：CVE + 停止维护
-     "vite-plugin-imagemin": ...,            // ❌ 移除：废弃
+     "vite-plugin-image-optimizer": ...,     // 替代方案
-     "vite-plugin-eslint": ...,              // ❌ 移除：不兼容 Vite 8
    },

-   "resolutions": {                          // ❌ 整个移除
-     "bin-wrapper": "npm:bin-wrapper-china",
-     "rollup": "^2.56.3",
-     "gifsicle": "5.2.0"
-   }
  }
```

---

## 七、Verification Plan

### Automated Tests
1. `npm ci` — 确认所有依赖可正常安装，无 peer dependency 冲突
2. `npm run build` — 确认生产构建通过
3. `npm audit --audit-level=high` — 确认无高危及以上漏洞
4. `npx vue-tsc --noEmit` — 确认 TypeScript 类型检查通过

### Manual Verification
1. 本地 `npm run dev` 启动开发服务器，验证页面可正常加载
2. 登录页面功能验证（密码传输链路变更）
3. 终端 (xterm) 组件渲染验证
4. 文件上传/下载功能验证
5. `docker compose up --build` 全流程集成测试

### Security Verification
1. 运行 `npm audit` 确认 0 个 critical/high 漏洞
2. 审查最终 `package-lock.json` 中无被 resolution 降级的包
3. 确认所有依赖来源为官方 npm registry
4. 确认 `node_modules` 中无 `bin-wrapper-china` 等非官方替换包

---

## 八、执行优先级建议

| 阶段 | 内容 | 预估工时 |
|------|------|----------|
| **P0 (立即)** | Axios 升级、移除 `resolutions`、移除 `mockjs` | 0.5h |
| **P1 (本周)** | Vite 3→8 升级 + Rollup 移除 + 插件适配 | 4-6h |
| **P2 (本周)** | 移除 `jsencrypt`/`ts-md5` + 密码传输链路重构 | 2-3h |
| **P3 (下周)** | ESLint 9 + TS 5 + Prettier 3 工具链现代化 | 2-3h |
| **P4 (下周)** | 供应链清理 (`@dangojs/*` 等) + 包体积优化 | 2-3h |
