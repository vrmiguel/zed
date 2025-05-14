-- Begin transaction to ensure atomicity
BEGIN;

-- Create temporary tables to store existing data
CREATE TEMPORARY TABLE temp_remote_projects AS 
SELECT * FROM remote_projects;

CREATE TEMPORARY TABLE temp_dev_servers AS 
SELECT * FROM dev_servers;

-- Drop foreign key constraints first to avoid conflicts
ALTER TABLE dev_servers DROP CONSTRAINT IF EXISTS dev_servers_channel_id_fkey;
ALTER TABLE remote_projects DROP CONSTRAINT IF EXISTS remote_projects_channel_id_fkey;

-- Perform schema changes
ALTER TABLE dev_servers DROP COLUMN channel_id;
ALTER TABLE dev_servers ADD COLUMN user_id INT NOT NULL REFERENCES users(id);

ALTER TABLE remote_projects DROP COLUMN channel_id;

-- Clean up temporary tables
DROP TABLE temp_remote_projects;
DROP TABLE temp_dev_servers;

COMMIT;