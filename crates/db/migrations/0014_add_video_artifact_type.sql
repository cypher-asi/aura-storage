-- Add 'video' to the artifacts type check constraint.
ALTER TABLE artifacts DROP CONSTRAINT artifacts_type_check;
ALTER TABLE artifacts ADD CONSTRAINT artifacts_type_check CHECK (type IN ('image', 'model', 'video'));
