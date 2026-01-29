CREATE TABLE contracts (
    address         BYTEA PRIMARY KEY,
    creator         BYTEA NOT NULL,
    creation_tx     BYTEA REFERENCES transactions(hash),
    bytecode        BYTEA,
    is_verified     BOOLEAN DEFAULT FALSE,
    name            TEXT,
    source_code     TEXT,
    abi             JSONB,
    compiler        TEXT,
    optimization    BOOLEAN,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
);
