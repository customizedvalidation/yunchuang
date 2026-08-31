-- Migration script for adding indexes to support performance optimization
-- Created: 2026-05-28
-- Target: PostgreSQL

-- ==============================================
-- Resource Table Indexes
-- ==============================================

-- Index for querying resources by cluster
CREATE INDEX IF NOT EXISTS idx_resources_cluster_id 
ON resources (cluster_id);

-- Index for querying resources by type
CREATE INDEX IF NOT EXISTS idx_resources_type 
ON resources (type);

-- Index for querying resources by status
CREATE INDEX IF NOT EXISTS idx_resources_status 
ON resources (status);

-- Composite index for common resource queries: by cluster and status
CREATE INDEX IF NOT EXISTS idx_resources_cluster_status 
ON resources (cluster_id, status);

-- Composite index for querying resources by type and status
CREATE INDEX IF NOT EXISTS idx_resources_type_status 
ON resources (type, status);

-- ==============================================
-- Job Table Indexes (Additional composite indexes)
-- ==============================================

-- Composite index for querying jobs by status and priority
-- This is critical for job scheduling and prioritization
CREATE INDEX IF NOT EXISTS idx_jobs_status_priority 
ON jobs (status, priority);

-- Composite index for querying jobs by tenant and status
-- Useful for multi-tenant scenarios
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_status 
ON jobs (tenant_id, status);

-- Composite index for querying jobs by cluster and status
CREATE INDEX IF NOT EXISTS idx_jobs_cluster_status 
ON jobs (cluster_id, status);

-- ==============================================
-- User Table Indexes
-- ==============================================

-- Index for querying users by email (already may exist but ensure it's there)
CREATE INDEX IF NOT EXISTS idx_users_email 
ON users (email);

-- ==============================================
-- Verification queries
-- ==============================================

-- To verify all indexes are created, run:
-- SELECT indexrelname, relname AS table_name 
-- FROM pg_indexes 
-- WHERE schemaname = 'public' 
-- AND relname IN ('resources', 'jobs', 'users');

-- To check index usage (after some traffic):
-- SELECT idx.relname AS index_name,
--        tbl.relname AS table_name,
--        idx_scan AS index_scans,
--        idx_tup_read AS tuples_read,
--        idx_tup_fetch AS tuples_fetched
-- FROM pg_stat_user_indexes idx
-- JOIN pg_class tbl ON idx.schemaname = tbl.schemaname AND idx.relname = tbl.relname
-- WHERE tbl.relname IN ('resources', 'jobs', 'users');
