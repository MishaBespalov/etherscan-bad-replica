// CREATE TABLE users (
//     id              UUID PRIMARY KEY,
//     email           TEXT UNIQUE NOT NULL,
//     password_hash   TEXT NOT NULL,
//     created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
// );
//
// -- API Keys
// CREATE TABLE api_keys (
//     id              UUID PRIMARY KEY,
//     user_id         UUID REFERENCES users(id),
//     key_hash        TEXT UNIQUE NOT NULL,
//     name            TEXT,
//     rate_limit      INT DEFAULT 100,
//     created_at      TIMESTAMPTZ DEFAULT NOW(), -- Changed
//     last_used_at    TIMESTAMPTZ                -- Changed
// );
//
// -- Usage tracking
// CREATE TABLE api_usage (
//     id              BIGSERIAL PRIMARY KEY,
//     api_key_id      UUID REFERENCES api_keys(id),
//     endpoint        TEXT NOT NULL,
//     timestamp       TIMESTAMPTZ DEFAULT NOW()  -- Changed
// );
// ```
//
