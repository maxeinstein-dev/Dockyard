-- Migration 5: spacers (spec 0022). STRICT, like the others, so a value of
-- the wrong type is refused.

-- One row per spacer: an empty room between two items of the dock, small
-- (a quarter of an icon) or large (half of one). Its place is where its key
-- (`spacer:<id>`) is in `workspace_items`; it belongs to the one workspace
-- that has it, and leaves this table when it leaves that workspace.
CREATE TABLE spacers (
  id TEXT PRIMARY KEY NOT NULL,      -- UUID v7, never reused
  size TEXT NOT NULL CHECK (size IN ('small', 'large')),
  created_at INTEGER NOT NULL        -- Unix time in milliseconds
) STRICT;
