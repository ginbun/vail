-- V1__init_schema.sql
-- 初始表结构

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

-- 主机表
CREATE TABLE host (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    hostname VARCHAR(128) NOT NULL,
    port INT DEFAULT 22,
    username VARCHAR(64),
    credential_type VARCHAR(16),
    credential_data TEXT,
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
    status SMALLINT DEFAULT 0,
    start_time TIMESTAMP DEFAULT NOW(),
    end_time TIMESTAMP,
    create_time TIMESTAMP DEFAULT NOW()
);

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
    chunk_size BIGINT DEFAULT 1048576,
    uploaded_size BIGINT DEFAULT 0,
    status SMALLINT DEFAULT 0,
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

-- 缓存表 (UNLOGGED)
CREATE UNLOGGED TABLE cache (
    cache_key VARCHAR(128) PRIMARY KEY,
    cache_value TEXT NOT NULL,
    expire_time TIMESTAMP,
    create_time TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_cache_expire ON cache(expire_time) WHERE expire_time IS NOT NULL;

-- 索引
CREATE INDEX idx_sys_user_username ON sys_user(username);
CREATE INDEX idx_host_name ON host(name);
CREATE INDEX idx_upload_task_task_no ON upload_task(task_no);
CREATE INDEX idx_upload_task_status ON upload_task(status);
CREATE INDEX idx_ssh_session_session_id ON ssh_session(session_id);
