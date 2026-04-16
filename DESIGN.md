# Vail - 下一代运维堡垒机

> 基于 Rust 后端重构 + Orion 前端兼容的现代化运维堡垒机系统

## 一、项目概述

### 1.1 背景

Vail 是对 [Orion Visor](https://github.com/dromara/orion-visor) 的后端重构方案：保留 Rust 技术路线重写后端核心能力，并以 Orion 前端兼容为目标，优先保障功能可迁移、交互可复用、上线风险可控。

### 1.2 技术栈

| 层级 | 技术选型 | 理由 |
|------|----------|------|
| 后端 Web 框架 | **Axum** | 现代异步框架，与 Tokio 原生集成，API 设计优雅 |
| ORM | **SeaORM** | 强大的类型安全查询构建器，支持 PostgreSQL |
| 异步运行时 | **Tokio** | Rust 异步事实标准 |
| 数据库 | **PostgreSQL** | 支持分区表、JSON、向量等高级特性 |
| 缓存 | **PostgreSQL UNLOGGED TABLE** | 无日志写入，极高性能，适合缓存场景 |
| SSH/SFTP | **ssh2-rs** | 纯 Rust SSH 库，支持断点续传 |
| 前端兼容策略 | **Orion UI 兼容层 (Vue 3 + Arco Design)** | 复用 Orion 成熟前端能力，降低替换成本与培训成本 |
| 前端构建工具 | **Vite (兼容 Orion 配置)** | 与 Orion 现有工程保持一致，降低迁移复杂度 |
| 数据库迁移 | **sqlx-cli (Flyway)** | 应用启动时自动迁移 |

### 1.3 核心特性

- **SSH/SFTP 断点续传**: 支持大文件分片上传，字节偏移断点续传
- **声明式分区表**: 登录日志、操作日志采用 PostgreSQL 分区表
- **高性能缓存**: 使用 UNLOGGED 表作为缓存层
- **自动数据迁移**: 应用启动时自动执行 SQL 迁移脚本
- **Orion 前端兼容**: API、字段语义、错误码与权限模型优先兼容 Orion UI
- **渐进式体验优化**: 在兼容基础上优化性能、可观测性与高频操作路径

---

## 二、系统架构

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                         Vail                                  │
├─────────────────────────┬───────────────────────────────────┤
│ vail-web (Orion Compatible) │    vail-rs (Rust/Axum)          │
│                         │                                     │
│   ┌────────────────┐    │   ┌─────────────┐  ┌───────────┐  │
│   │ Vue Router      │    │   │   API       │  │  SSH/SFTP │  │
│   │ Pinia Stores    │    │   │   Layer     │  │  Module   │  │
│   │ Arco UI         │    │   └─────────────┘  └───────────┘  │
│   │ API Adapter     │    │   ┌─────────────┐  ┌───────────┐  │
│   └────────────────┘    │   │   Service    │  │  Auth     │  │
│                         │   │   Layer       │  │  Module   │  │
│                         │   └─────────────┘  └───────────┘  │
│                         │   ┌─────────────┐                 │
│                         │   │  Database    │                 │
│                         │   │  Layer       │                 │
│                         │   └─────────────┘                 │
└─────────────────────────┴───────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  PostgreSQL      │
                    │  - 分区表        │
                    │  - UNLOGGED     │
                    └─────────────────┘
```

### 2.2 项目结构

```
vail/
├── vail-rs/                     # Rust 后端
│   ├── Cargo.toml
│   ├── migrations/              # SQL 迁移脚本
│   │   └── *.sql
│   └── src/
│       ├── main.rs              # 程序入口
│       ├── lib.rs               
│       ├── api/                 # HTTP API 层
│       │   ├── mod.rs
│       │   ├── auth.rs          # 认证接口
│       │   ├── host.rs          # 主机管理
│       │   ├── sftp.rs          # SFTP 上传
│       │   └── mod.rs
│       ├── db/                  # 数据库层
│       │   ├── mod.rs
│       │   ├── entities/        # SeaORM Entity
│       │   └── cache.rs         # 缓存操作
│       ├── service/             # 业务逻辑层
│       │   ├── mod.rs
│       │   ├── auth.rs
│       │   ├── ssh.rs
│       │   └── sftp.rs
│       ├── model/               # DTO/VO
│       ├── middleware/          # 中间件 (JWT/日志)
│       └── utils/               # 工具函数
│
├── vail-web/                    # Orion 兼容前端 (Vue 3)
│   ├── package.json
│   ├── config/
│   │   ├── vite.config.dev.ts
│   │   └── vite.config.prod.ts
│   ├── tsconfig.json
│   ├── src/
│   │   ├── main.ts
│   │   ├── views/               # 页面
│   │   ├── components/          # 组件
│   │   ├── router/              # 路由
│   │   ├── store/               # Pinia 状态
│   │   ├── api/                 # API 请求封装
│   │   ├── adapter/             # Orion 接口/字段兼容层
│   │   └── styles/
│   └── static/
│
└── README.md
```

---

## 三、数据库设计

### 3.1 核心表结构 (V1__init_schema.sql)

#### 用户与权限

```sql
-- 用户表
CREATE TABLE sys_user (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(32) NOT NULL UNIQUE,
    password VARCHAR(128) NOT NULL,
    nickname VARCHAR(64),
    email VARCHAR(128),
    phone VARCHAR(32),
    avatar VARCHAR(256),
    status SMALLINT DEFAULT 1,
    last_login_time TIMESTAMP,
    last_login_ip VARCHAR(64),
    create_time TIMESTAMP DEFAULT NOW(),
    update_time TIMESTAMP DEFAULT NOW(),
    deleted SMALLINT DEFAULT 0
);

-- 角色表
CREATE TABLE sys_role (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL,
    code VARCHAR(64) NOT NULL UNIQUE,
    description VARCHAR(256),
    status SMALLINT DEFAULT 1,
    create_time TIMESTAMP DEFAULT NOW(),
    deleted SMALLINT DEFAULT 0
);

-- 用户角色关联
CREATE TABLE sys_user_role (
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    create_time TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

-- 菜单表
CREATE TABLE sys_menu (
    id BIGSERIAL PRIMARY KEY,
    parent_id BIGINT DEFAULT 0,
    name VARCHAR(64) NOT NULL,
    path VARCHAR(128),
    component VARCHAR(128),
    icon VARCHAR(64),
    type SMALLINT DEFAULT 1,
    sort INT DEFAULT 0,
    visible SMALLINT DEFAULT 1,
    permission VARCHAR(128),
    create_time TIMESTAMP DEFAULT NOW()
);

-- 角色菜单关联
CREATE TABLE sys_role_menu (
    role_id BIGINT NOT NULL,
    menu_id BIGINT NOT NULL,
    PRIMARY KEY (role_id, menu_id)
);
```

#### 资产与主机

```sql
-- 主机表
CREATE TABLE host (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    hostname VARCHAR(128) NOT NULL,
    port INT DEFAULT 22,
    username VARCHAR(64),
    credential_type VARCHAR(16),      -- 'password' | 'private_key'
    credential_data TEXT,            -- 加密存储的凭证
    description VARCHAR(512),
    tags JSONB,
    status SMALLINT DEFAULT 1,
    create_time TIMESTAMP DEFAULT NOW(),
    update_time TIMESTAMP DEFAULT NOW(),
    deleted SMALLINT DEFAULT 0
);

-- 主机组
CREATE TABLE host_group (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    parent_id BIGINT DEFAULT 0,
    description VARCHAR(256),
    sort INT DEFAULT 0,
    create_time TIMESTAMP DEFAULT NOW(),
    deleted SMALLINT DEFAULT 0
);

-- 主机主机组关联
CREATE TABLE host_group_rel (
    host_id BIGINT NOT NULL,
    group_id BIGINT NOT NULL,
    PRIMARY KEY (host_id, group_id)
);

-- SSH 会话记录
CREATE TABLE ssh_session (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    session_id VARCHAR(64) NOT NULL UNIQUE,
    status SMALLINT DEFAULT 0,        -- 0:connecting 1:connected 2:disconnected
    start_time TIMESTAMP DEFAULT NOW(),
    end_time TIMESTAMP,
    create_time TIMESTAMP DEFAULT NOW()
);
```

#### 文件传输

```sql
-- SFTP 上传任务
CREATE TABLE upload_task (
    id BIGSERIAL PRIMARY KEY,
    task_no VARCHAR(64) NOT NULL UNIQUE,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    remote_path VARCHAR(512) NOT NULL,
    file_name VARCHAR(256),
    file_size BIGINT,
    file_md5 VARCHAR(32),
    chunk_size BIGINT DEFAULT 1048576,  -- 1MB
    uploaded_size BIGINT DEFAULT 0,
    status SMALLINT DEFAULT 0,           -- 0:pending 1:uploading 2:completed 3:failed
    error_message TEXT,
    create_time TIMESTAMP DEFAULT NOW(),
    update_time TIMESTAMP DEFAULT NOW()
);

-- SFTP 下载任务
CREATE TABLE download_task (
    id BIGSERIAL PRIMARY KEY,
    task_no VARCHAR(64) NOT NULL UNIQUE,
    user_id BIGINT NOT NULL,
    host_id BIGINT NOT NULL,
    remote_path VARCHAR(512) NOT NULL,
    local_path VARCHAR(512),
    file_name VARCHAR(256),
    file_size BIGINT,
    downloaded_size BIGINT DEFAULT 0,
    status SMALLINT DEFAULT 0,
    error_message TEXT,
    create_time TIMESTAMP DEFAULT NOW(),
    update_time TIMESTAMP DEFAULT NOW()
);
```

#### 缓存表

```sql
-- 缓存表 (UNLOGGED - 无日志写入，高性能)
CREATE UNLOGGED TABLE cache (
    cache_key VARCHAR(128) PRIMARY KEY,
    cache_value TEXT NOT NULL,
    expire_time TIMESTAMP,
    create_time TIMESTAMP DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_cache_expire ON cache(expire_time) WHERE expire_time IS NOT NULL;
```

### 3.2 分区表设计

#### 登录日志 (V2__partition_login_log.sql)

```sql
-- 登录日志 - 声明式分区 (月度)
CREATE TABLE login_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    ip VARCHAR(64),
    location VARCHAR(128),
    user_agent VARCHAR(256),
    result SMALLINT,
    error_message TEXT,
    create_time TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (create_time);

-- 默认分区 (当月)
CREATE TABLE login_log_default PARTITION OF login_log
    FOR VALUES FROM (MINVALUE) TO (MAXVALUE);

-- 自动分区函数
CREATE OR REPLACE FUNCTION create_login_log_partition()
RETURNS void AS $$
DECLARE
    current_month TEXT;
    partition_name TEXT;
BEGIN
    current_month := to_char(NOW(), 'YYYY_MM');
    partition_name := 'login_log_' || current_month;
    
    IF NOT EXISTS (
        SELECT 1 FROM pg_tables 
        WHERE tablename = partition_name
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF login_log FOR VALUES FROM (%L) TO (%L)',
            partition_name,
            date_trunc('month', NOW()),
            date_trunc('month', NOW() + interval '1 month')
        );
    END IF;
END;
$$ LANGUAGE plpgsql;
```

#### 操作日志 (V3__partition_operator_log.sql)

```sql
-- 操作日志 - 声明式分区 (月度)
CREATE TABLE operator_log (
    id BIGSERIAL,
    user_id BIGINT,
    username VARCHAR(32),
    module VARCHAR(32),
    operation VARCHAR(64),
    method VARCHAR(16),
    path VARCHAR(256),
    params JSONB,
    result SMALLINT,
    error_message TEXT,
    duration INT,
    trace_id VARCHAR(64),
    ip VARCHAR(64),
    user_agent VARCHAR(256),
    create_time TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (create_time);

-- 默认分区
CREATE TABLE operator_log_default PARTITION OF operator_log
    FOR VALUES FROM (MINVALUE) TO (MAXVALUE);

-- 自动分区函数
CREATE OR REPLACE FUNCTION create_operator_log_partition()
RETURNS void AS $$
DECLARE
    current_month TEXT;
    partition_name TEXT;
BEGIN
    current_month := to_char(NOW(), 'YYYY_MM');
    partition_name := 'operator_log_' || current_month;
    
    IF NOT EXISTS (
        SELECT 1 FROM pg_tables 
        WHERE tablename = partition_name
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF operator_log FOR VALUES FROM (%L) TO (%L)',
            partition_name,
            date_trunc('month', NOW()),
            date_trunc('month', NOW() + interval '1 month')
        );
    END IF;
END;
$$ LANGUAGE plpgsql;
```

---

## 四、核心功能设计

### 4.0 前端兼容设计原则

- **兼容优先**: 前端路由结构、页面信息架构、权限展示语义与 Orion 保持一致，降低迁移阻力。
- **契约稳定**: 后端优先提供 Orion 可消费的 API 形态；新增能力通过可选字段或新版本接口扩展。
- **渐进优化**: 仅在不破坏兼容性的前提下优化加载速度、交互反馈和批量操作效率。
- **可回退**: 关键功能保留兼容开关，支持分模块灰度上线与快速回滚。

### 4.1 认证模块

#### JWT 认证流程

```
1. 用户登录 → /api/auth/login
   - 验证用户名密码
   - 生成 JWT Token (access_token)
   - 生成刷新 Token (refresh_token)
   - 记录登录日志

2. 请求鉴权
   - Header: Authorization: Bearer <token>
   - Middleware 验证 Token
   - 解析用户信息放入 Request State

3. Token 刷新
   - /api/auth/refresh
   - 验证 refresh_token
   - 生成新的 access_token
```

#### 密码加密

- 使用 **Argon2** 密码哈希
- 存储格式: `$argon2id$v=19$m=65536,t=2,p=4$...`

### 4.2 SSH/SFTP 模块

#### 连接管理

```rust
// SSH 连接池
struct SshPool {
    connections: HashMap<i64, SshConnection>,  // host_id -> connection
}

// 连接获取
async fn get_ssh_connection(host_id: i64) -> Result<SshConnection>
```

#### SFTP 断点续传

```
前端分片逻辑:
┌─────────────────────────────────────────────────────────────┐
│  1. 文件选择                                                 │
│     - file: File                                            │
│     - chunkSize: 1MB                                        │
│     - chunks = Math.ceil(file.size / chunkSize)            │
│                                                             │
│  2. 遍历上传每个分片                                         │
│     for (let i = 0; i < chunks; i++) {                     │
│       offset = i * chunkSize                                │
│       chunk = file.slice(offset, offset + chunkSize)       │
│       await uploadChunk({ chunk, offset, taskId })         │
│     }                                                       │
│                                                             │
│  3. 完成后通知服务端合并                                      │
│     await uploadComplete({ taskId })                       │
└─────────────────────────────────────────────────────────────┘
```

```
后端处理逻辑:
┌─────────────────────────────────────────────────────────────┐
│  1. 创建上传任务                                             │
│     POST /api/sftp/task                                     │
│     - 返回 taskNo, taskId                                   │
│                                                             │
│  2. 分片上传                                                 │
│     POST /api/sftp/upload                                   │
│     - taskId, chunkIndex, offset, content                   │
│     - 写入临时文件 tmp_<taskNo>.part.<chunkIndex>          │
│     - 更新 uploadTask.uploadedSize                         │
│                                                             │
│  3. 合并分片                                                 │
│     POST /api/sftp/complete                                │
│     - 顺序合并分片文件                                       │
│     - 移动到目标路径                                         │
│     - 更新任务状态为 completed                              │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 缓存设计

#### UNLOGGED 表优势

- 无 WAL 日志写入
- 写性能提升 10x+
- 适合频繁读写、不需要持久化的数据

#### 使用场景

```sql
-- Session 缓存
INSERT INTO cache (cache_key, cache_value, expire_time) 
VALUES ('session:abc123', '{"user_id": 1}', NOW() + INTERVAL '2 hours');

-- 限流计数
INSERT INTO cache (cache_key, cache_value, expire_time)
VALUES ('rate_limit:192.168.1.1', '10', NOW() + INTERVAL '1 minute');

-- 临时文件 token
INSERT INTO cache (cache_key, cache_value, expire_time)
VALUES ('upload:token:xyz', '{"task_id": 1}', NOW() + INTERVAL '1 hour');
```

---

## 五、API 设计

### 5.1 认证接口

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/auth/login | 用户登录 |
| POST | /api/auth/logout | 用户登出 |
| POST | /api/auth/refresh | 刷新 Token |
| GET | /api/auth/me | 获取当前用户信息 |

### 5.2 主机管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/hosts | 获取主机列表 |
| POST | /api/hosts | 创建主机 |
| GET | /api/hosts/:id | 获取主机详情 |
| PUT | /api/hosts/:id | 更新主机 |
| DELETE | /api/hosts/:id | 删除主机 |
| GET | /api/host-groups | 获取主机组列表 |
| POST | /api/host-groups | 创建主机组 |

### 5.3 SFTP 接口

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/sftp/task | 创建上传任务 |
| POST | /api/sftp/upload | 上传分片 |
| POST | /api/sftp/complete | 完成上传 |
| GET | /api/sftp/tasks | 获取上传任务列表 |
| GET | /api/sftp/tasks/:id | 获取任务详情 |

### 5.4 SSH 会话

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | /api/ssh/connect/:hostId | 创建 SSH 连接 |
| POST | /api/ssh/disconnect/:sessionId | 断开连接 |

### 5.5 Orion 兼容约束

- 鉴权与会话字段命名保持与 Orion UI 兼容，避免前端大面积改造。
- 错误码与错误消息结构统一（业务码 + 文案 + trace_id），便于前端统一处理。
- 分页、筛选、排序参数维持 Orion 习惯格式，减少页面层适配代码。
- 新增能力优先采用向后兼容扩展字段，不直接破坏现有接口。

---

## 六、实施路线图

### Phase 1: 基础设施 (Week 1)

- [ ] 项目骨架搭建 (Rust + Orion 兼容前端)
- [ ] 数据库迁移框架
- [ ] 基础配置管理
- [ ] 日志系统

### Phase 2: 认证模块 (Week 2)

- [ ] 用户注册/登录
- [ ] JWT 认证
- [ ] 登录日志记录
- [ ] 分区表自动创建

### Phase 3: SSH/SFTP (Week 3-4)

- [ ] SSH 连接管理
- [ ] SFTP 文件上传 (断点续传)
- [ ] SFTP 文件下载
- [ ] WebSocket 实时进度

### Phase 4: 资产管理 (Week 5)

- [ ] 主机 CRUD
- [ ] 主机组管理
- [ ] 主机连接测试

### Phase 5: 权限管理 (Week 6)

- [ ] 角色管理
- [ ] 菜单管理
- [ ] 权限控制

### Phase 6: 操作日志 (Week 7)

- [ ] 操作日志记录
- [ ] 日志查询
- [ ] 分区表维护

### Phase 7: 前端兼容与体验优化 (Week 8)

- [ ] Orion 前端关键页面兼容验证（登录/主机/上传/日志）
- [ ] API 兼容适配层收敛与清理
- [ ] 高频操作路径体验优化（批量、搜索、筛选）
- [ ] 兼容回归测试与灰度发布准备

---

## 七、版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| v0.1.0 | 2026-04-14 | 初始设计文档 |
| v0.2.0 | 2026-04-16 | 调整为 Rust 后端重构 + Orion 前端兼容方向 |

---

## 八、参考链接

- [Axum](https://github.com/tokio-rs/axum)
- [SeaORM](https://www.sea-ql.org/SeaORM/)
- [ssh2-rs](https://github.com/alexcrichton/ssh2-rs)
- [Vue 3](https://vuejs.org/)
- [Arco Design Vue](https://arco.design/vue)
- [PostgreSQL Partitioning](https://www.postgresql.org/docs/current/ddl-partitioning.html)
