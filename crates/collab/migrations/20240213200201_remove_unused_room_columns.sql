-- Migration to remove unused room columns
-- First create backup tables and copy data
BEGIN;

-- Create backup table for rooms
CREATE TABLE rooms_backup_20240213 AS
SELECT enviroment, environment
FROM rooms
WHERE enviroment IS NOT NULL 
   OR environment IS NOT NULL;

-- Create backup table for room_participants
CREATE TABLE room_participants_backup_20240213 AS 
SELECT in_call
FROM room_participants 
WHERE in_call IS NOT NULL;

-- Verify columns are unused (no non-null values)
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM rooms 
    WHERE enviroment IS NOT NULL 
       OR environment IS NOT NULL
  ) THEN
    RAISE EXCEPTION 'Cannot drop columns - found non-null values in rooms.enviroment or rooms.environment';
  END IF;

  IF EXISTS (
    SELECT 1 FROM room_participants
    WHERE in_call IS NOT NULL
  ) THEN
    RAISE EXCEPTION 'Cannot drop columns - found non-null values in room_participants.in_call';
  END IF;
END $$;

-- Remove columns if verification passes
ALTER TABLE rooms DROP COLUMN enviroment;
ALTER TABLE rooms DROP COLUMN environment;
ALTER TABLE room_participants DROP COLUMN in_call;

-- Add comment noting backup tables
COMMENT ON TABLE rooms_backup_20240213 IS 'Backup of dropped columns from rooms table - migration 20240213200201';
COMMENT ON TABLE room_participants_backup_20240213 IS 'Backup of dropped columns from room_participants table - migration 20240213200201';

COMMIT;