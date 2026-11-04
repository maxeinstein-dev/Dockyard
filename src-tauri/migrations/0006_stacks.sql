-- Migration 6: stacks of widgets (spec 0023). STRICT, like the others, so a
-- value of the wrong type is refused.

-- One row per stack: two or more widgets in one tile of the dock, the one
-- at `top` (an index of its members, in their order) shown. Its place is
-- where its key (`stack:<id>`) is in `workspace_items`; it belongs to the
-- one workspace that has it, and leaves this table when it leaves that
-- workspace.
CREATE TABLE stacks (
  id TEXT PRIMARY KEY NOT NULL,      -- UUID v7, never reused
  top INTEGER NOT NULL,              -- 0-based, among its members
  created_at INTEGER NOT NULL        -- Unix time in milliseconds
) STRICT;

-- The widgets of each stack, in order. A widget removed for good leaves its
-- stacks with it.
CREATE TABLE stack_members (
  stack_id TEXT NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
  widget_id TEXT NOT NULL REFERENCES widget_instances(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,         -- 0-based, contiguous within the stack
  PRIMARY KEY (stack_id, widget_id)
) STRICT;
