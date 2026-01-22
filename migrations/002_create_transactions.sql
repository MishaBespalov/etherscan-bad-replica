CREATE TABLE transactions (
    hash            BYTEA PRIMARY KEY,
    block_number    BIGINT REFERENCES blocks(number),
    tx_index        INT NOT NULL,
    from_addr       BYTEA NOT NULL,
    to_addr         BYTEA,
    value           BYTEA NOT NULL,
    gas_price       BIGINT NOT NULL,
    gas_limit       BIGINT NOT NULL,
    gas_used        BIGINT NOT NULL,
    input           BYTEA,
    nonce           BIGINT NOT NULL,
    status          SMALLINT,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
);
