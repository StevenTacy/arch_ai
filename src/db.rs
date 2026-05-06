use sqlx::PgPool;

use crate::error::AppError;

#[derive(sqlx::FromRow)]
pub struct LawChunk {
    law_name: String,
    article_number: String,
    content: String,
}

/// Full-text search over law_chunks using PostgreSQL tsvector.
/// Returns up to `top_k` chunks ranked by relevance. Returns empty vec on blank query.
pub async fn search_law(pool: &PgPool, query: &str, top_k: i64) -> Result<Vec<LawChunk>, AppError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, LawChunk>(
        "SELECT law_name, article_number, content \
         FROM law_chunks \
         WHERE tsv @@ plainto_tsquery('simple', $1) \
         ORDER BY ts_rank(tsv, plainto_tsquery('simple', $1)) DESC \
         LIMIT $2",
    )
    .bind(query)
    .bind(top_k)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub fn format_chunks(chunks: &[LawChunk]) -> String {
    chunks
        .iter()
        .map(|c| format!("【{}{}】\n{}", c.law_name, c.article_number, c.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}
