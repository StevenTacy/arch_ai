CREATE TABLE IF NOT EXISTS law_chunks (
    id             BIGSERIAL PRIMARY KEY,
    law_name       TEXT NOT NULL,
    article_number TEXT NOT NULL,
    content        TEXT NOT NULL,
    -- Generated tsvector: 'simple' dictionary keeps tokens as-is (preserves CJK)
    tsv            TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('simple', law_name || ' ' || article_number || ' ' || content)
    ) STORED
);

-- GIN index for fast full-text lookups
CREATE INDEX IF NOT EXISTS law_chunks_tsv_idx ON law_chunks USING gin(tsv);
