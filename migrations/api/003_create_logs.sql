CREATE TABLE logs (
    id              BIGSERIAL PRIMARY KEY,
    block_number    BIGINT NOT NULL,
    tx_hash         BYTEA REFERENCES transactions(hash),
    log_index       INT NOT NULL,
    address         BYTEA NOT NULL,
    topic0          BYTEA,
    topic1          BYTEA,
    topic2          BYTEA,
    topic3          BYTEA,
    data            BYTEA,
    UNIQUE(tx_hash, log_index)
);
