-- Backup tables before dropping columns
CREATE TABLE IF NOT EXISTS rooms_backup_20240213 AS SELECT * FROM rooms;
CREATE TABLE IF NOT EXISTS room_participants_backup_20240213 AS SELECT * FROM room_participants;

-- Check for non-null values in columns before dropping
DO $$
DECLARE
    env_count INTEGER;
    envir_count INTEGER;
    call_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO env_count FROM rooms WHERE environment IS NOT NULL;
    SELECT COUNT(*) INTO envir_count FROM rooms WHERE enviroment IS NOT NULL;
    SELECT COUNT(*) INTO call_count FROM room_participants WHERE in_call IS NOT NULL;
    
    IF env_count > 0 OR envir_count > 0 OR call_count > 0 THEN
        RAISE EXCEPTION 'Cannot drop columns - they contain data. Review data before proceeding.';
    END IF;
END $$;

-- Drop unused columns after validation
-- The 'enviroment' column was a typo and never used
-- The 'environment' column was replaced by release_channel
-- The 'in_call' status is now tracked in a separate system
ALTER TABLE rooms DROP COLUMN IF EXISTS enviroment;
ALTER TABLE rooms DROP COLUMN IF EXISTS environment;
ALTER TABLE room_participants DROP COLUMN IF EXISTS in_call;