-- Verify columns are empty/unused before dropping
DO $$
BEGIN
    -- Check for any non-null values in environment columns
    IF EXISTS (
        SELECT 1 FROM rooms 
        WHERE enviroment IS NOT NULL 
        OR environment IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Cannot drop environment columns - they contain data';
    END IF;

    -- Check for any active calls
    IF EXISTS (
        SELECT 1 FROM room_participants 
        WHERE in_call = true
    ) THEN
        RAISE EXCEPTION 'Cannot drop in_call column - there are active calls';
    END IF;
END $$;

-- Create backup tables
CREATE TABLE rooms_backup_20240213 AS SELECT * FROM rooms;
CREATE TABLE room_participants_backup_20240213 AS SELECT * FROM room_participants;

-- Drop columns after verification
ALTER TABLE rooms DROP COLUMN enviroment;
ALTER TABLE rooms DROP COLUMN environment;
ALTER TABLE room_participants DROP COLUMN in_call;

-- Add comment explaining why these columns were removed
COMMENT ON TABLE rooms IS 'Removed duplicate environment columns and unused in_call column - tracked in ticket 1ae26f61';