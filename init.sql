-- Metaclouds Database Initialization Script
-- Create tables for Metaclouds system

-- User table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    tenant_id INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Cluster table
CREATE TABLE IF NOT EXISTS clusters (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    nodes INTEGER NOT NULL DEFAULT 0,
    gpus INTEGER NOT NULL DEFAULT 0,
    cpus INTEGER NOT NULL DEFAULT 0,
    memory INTEGER NOT NULL DEFAULT 0,
    storage INTEGER NOT NULL DEFAULT 0,
    network_type VARCHAR(50),
    location VARCHAR(100),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Resource table
CREATE TABLE IF NOT EXISTS resources (
    id SERIAL PRIMARY KEY,
    cluster_id INTEGER REFERENCES clusters(id),
    name VARCHAR(100) NOT NULL,
    type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'available',
    total INTEGER NOT NULL DEFAULT 0,
    used INTEGER NOT NULL DEFAULT 0,
    available INTEGER NOT NULL DEFAULT 0,
    utilization DOUBLE PRECISION NOT NULL DEFAULT 0,
    details TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Job table
CREATE TABLE IF NOT EXISTS jobs (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    gpus INTEGER NOT NULL DEFAULT 0,
    cpus INTEGER NOT NULL DEFAULT 0,
    memory INTEGER NOT NULL DEFAULT 0,
    duration INTEGER NOT NULL DEFAULT 0,
    cluster_id INTEGER REFERENCES clusters(id),
    tenant_id INTEGER,
    user_id INTEGER REFERENCES users(id),
    progress INTEGER NOT NULL DEFAULT 0,
    output_path TEXT,
    error_msg TEXT,
    scheduled_at TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tenant table
CREATE TABLE IF NOT EXISTS tenants (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    gpu_quota INTEGER NOT NULL DEFAULT 0,
    cpu_quota INTEGER NOT NULL DEFAULT 0,
    memory_quota INTEGER NOT NULL DEFAULT 0,
    storage_quota INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Acceleration Suite table
CREATE TABLE IF NOT EXISTS acceleration_suites (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    type VARCHAR(20) NOT NULL,
    version VARCHAR(50),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    details TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Security Policy table
CREATE TABLE IF NOT EXISTS security_policies (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    rules TEXT,
    details TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Alert table
CREATE TABLE IF NOT EXISTS alerts (
    id SERIAL PRIMARY KEY,
    type VARCHAR(20) NOT NULL,
    level VARCHAR(20) NOT NULL DEFAULT 'warning',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    message TEXT NOT NULL,
    details TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Schedule table for job scheduling
CREATE TABLE IF NOT EXISTS schedules (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    cron_expression VARCHAR(100) NOT NULL,
    job_id INTEGER REFERENCES jobs(id),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_run TIMESTAMP,
    next_run TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert default data
-- 注意：下方 admin 用户的 bcrypt 哈希必须与 .env 的 DEFAULT_ADMIN_PASSWORD 保持一致；
--       每次轮换 DEFAULT_ADMIN_PASSWORD 后须用相同算法重新生成此哈希（见凭据轮换流程）。
INSERT INTO users (username, email, password, role, tenant_id) 
VALUES ('admin', 'admin@metaclouds.com', '$2a$10$0/mEjQUVuYO6h8S.JQPXo.VFgvhKZ8skphTLDcnftkMg10H74mB4Ka', 'admin', 0)
ON CONFLICT (username) DO NOTHING;

INSERT INTO tenants (name, description, gpu_quota, cpu_quota, memory_quota, storage_quota)
VALUES ('Default Tenant', 'Default system tenant', 100, 1000, 10000, 10000)
ON CONFLICT (name) DO NOTHING;

INSERT INTO clusters (name, description, nodes, gpus, cpus, memory, storage, network_type, location)
VALUES 
    ('GPU-Cluster-1', 'Primary GPU cluster for deep learning', 5, 40, 256, 2048, 100, 'InfiniBand', 'US-West'),
    ('CPU-Cluster-1', 'CPU cluster for general computing', 10, 0, 512, 4096, 200, 'Ethernet', 'US-East')
ON CONFLICT (name) DO NOTHING;

-- Insert sample GPU resources
DO $$
BEGIN
    FOR i IN 0..39 LOOP
        INSERT INTO resources (cluster_id, name, type, status, total, used, available, utilization, details)
        VALUES (1, 'NVIDIA-A100-' || i, 'gpu', 'available', 1, 0, 1, 0, 'NVIDIA A100 80GB');
    END LOOP;
END $$;

-- Insert sample job
INSERT INTO jobs (name, description, type, status, gpus, cpus, memory, cluster_id, tenant_id, user_id, progress)
VALUES 
    ('training-job-1', 'Deep learning training job', 'training', 'running', 4, 32, 128, 1, 1, 1, 65),
    ('inference-job-1', 'Model inference job', 'inference', 'completed', 1, 8, 32, 1, 1, 1, 100)
ON CONFLICT DO NOTHING;

-- Insert acceleration suite
INSERT INTO acceleration_suites (name, description, type, version, enabled, details)
VALUES ('CUDA-11.7', 'NVIDIA CUDA 11.7 toolkit', 'cuda', '11.7.1', TRUE, '{"framework": "PyTorch", "compute_capability": "8.0"}')
ON CONFLICT (name) DO NOTHING;

-- Insert security policy
INSERT INTO security_policies (name, description, type, enabled, rules, details)
VALUES ('network-isolation', 'Enable network isolation between tenants', 'network', TRUE, '{"ingress": "deny-all", "egress": "allow-local"}', 'Network isolation policy')
ON CONFLICT (name) DO NOTHING;

-- Insert sample alert
INSERT INTO alerts (type, level, status, message, details)
VALUES ('resource', 'warning', 'active', 'GPU utilization exceeds 90%', '{"gpu_id": "NVIDIA-A100-0", "utilization": 95}');
