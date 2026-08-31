#!/bin/bash
# Migration script runner for PostgreSQL
# Usage: ./run_migration.sh <database_host> <database_port> <database_name> <database_user> <database_password>

set -e

# Check if all arguments are provided
if [ $# -ne 5 ]; then
    echo "Usage: $0 <database_host> <database_port> <database_name> <database_user> <database_password>"
    exit 1
fi

DB_HOST="$1"
DB_PORT="$2"
DB_NAME="$3"
DB_USER="$4"
DB_PASSWORD="$5"

MIGRATION_FILE="20260528_add_indexes.sql"
MIGRATION_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$MIGRATION_FILE"

echo "=========================================="
echo "Running database migration"
echo "Migration file: $MIGRATION_PATH"
echo "Target database: $DB_HOST:$DB_PORT/$DB_NAME"
echo "=========================================="

# Set PGPASSWORD environment variable for psql
export PGPASSWORD="$DB_PASSWORD"

# Run the migration
psql -h "$DB_HOST" -p "$DB_PORT" -d "$DB_NAME" -U "$DB_USER" -f "$MIGRATION_PATH"

if [ $? -eq 0 ]; then
    echo ""
    echo "=========================================="
    echo "Migration completed successfully!"
    echo "=========================================="
else
    echo ""
    echo "=========================================="
    echo "Migration failed!"
    echo "=========================================="
    exit 1
fi
