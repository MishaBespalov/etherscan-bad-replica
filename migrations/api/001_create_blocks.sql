CREATE TABLE blocks (
    number          BIGINT PRIMARY KEY,
    hash            BYTEA UNIQUE NOT NULL,
    parent_hash     BYTEA NOT NULL,
    timestamp       TIMESTAMPTZ NOT NULL,     -- Changed: Stores block time (UTC)
    miner           BYTEA NOT NULL,
    gas_used        BIGINT NOT NULL,
    gas_limit       BIGINT NOT NULL,
    base_fee        BIGINT,
    tx_count        INT NOT NULL,
    size            INT NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed: DB insertion time
);
