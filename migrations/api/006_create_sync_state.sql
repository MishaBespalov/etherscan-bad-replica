CREATE TABLE sync_state (
    id              INT PRIMARY KEY DEFAULT 1,
    last_block      BIGINT NOT NULL,
    last_updated    TIMESTAMPTZ DEFAULT NOW() -- Changed
);
