-- Create backup tables
CREATE TABLE remote_projects_backup AS SELECT * FROM remote_projects;
CREATE TABLE dev_servers_backup AS SELECT * FROM dev_servers;

-- Drop foreign key constraints first to avoid issues during schema changes
ALTER TABLE dev_servers DROP CONSTRAINT IF EXISTS dev_servers_channel_id_fkey;
ALTER TABLE remote_projects DROP CONSTRAINT IF EXISTS remote_projects_channel_id_fkey;

-- Drop and add columns
ALTER TABLE dev_servers DROP COLUMN channel_id;
ALTER TABLE dev_servers ADD COLUMN user_id INT NOT NULL REFERENCES users(id);
ALTER TABLE remote_projects DROP COLUMN channel_id;

-- Delete records but keep the structure
TRUNCATE remote_projects CASCADE;
TRUNCATE dev_servers CASCADE;

-- Restore data that should be preserved
-- Note: You will need to implement the data migration logic based on your requirements
-- Example: INSERT INTO dev_servers (user_id, ...) SELECT user_id, ... FROM dev_servers_backup;