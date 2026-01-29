CREATE TABLE token_transfers (
    id              BIGSERIAL PRIMARY KEY,
    block_number    BIGINT NOT NULL,
    tx_hash         BYTEA NOT NULL,
    log_index       INT NOT NULL,
    token_address   BYTEA NOT NULL,
    from_addr       BYTEA NOT NULL,
    to_addr         BYTEA NOT NULL,
    value           BYTEA NOT NULL,
    token_id        BYTEA NOT NULL,
    token_type      SMALLINT NOT NULL,
    UNIQUE(tx_hash, log_index)
);
