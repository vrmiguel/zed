-- Create temporary tables to preserve data
CREATE TEMP TABLE temp_dev_servers AS SELECT * FROM dev_servers;
CREATE TEMP TABLE temp_remote_projects AS SELECT * FROM remote_projects;

-- Add new user_id column to dev_servers
ALTER TABLE dev_servers ADD COLUMN user_id INT;

-- Migrate data: Associate dev servers with channel owners
UPDATE dev_servers d
SET user_id = (
    SELECT owner_id 
    FROM channels c 
    WHERE c.id = d.channel_id
);

-- Add NOT NULL constraint after data migration
ALTER TABLE dev_servers 
    ALTER COLUMN user_id SET NOT NULL,
    ADD CONSTRAINT dev_servers_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id);

-- Drop old columns
ALTER TABLE dev_servers DROP COLUMN channel_id;
ALTER TABLE remote_projects DROP COLUMN channel_id;

-- Create backup tables with timestamp
CREATE TABLE backup_dev_servers_20240412 AS SELECT * FROM temp_dev_servers;
CREATE TABLE backup_remote_projects_20240412 AS SELECT * FROM temp_remote_projects;

-- Drop temporary tables
DROP TABLE temp_dev_servers;
DROP TABLE temp_remote_projects;

-- Add comment explaining backup tables
COMMENT ON TABLE backup_dev_servers_20240412 IS 'Backup of dev_servers table before migration to user-based model';
COMMENT ON TABLE backup_remote_projects_20240412 IS 'Backup of remote_projects table before migration to user-based model';