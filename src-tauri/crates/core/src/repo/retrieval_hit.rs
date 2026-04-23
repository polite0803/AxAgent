use sea_orm::*;

use crate::entity::retrieval_hits;
use crate::utils::gen_id;

/// Record a single retrieval hit.
pub async fn record_hit(
    db: &DatabaseConnection,
    conversation_id: &str,
    message_id: &str,
    knowledge_base_id: &str,
    document_id: &str,
    chunk_ref: &str,
    score: f64,
    preview: &str,
) -> Result<(), DbErr> {
    let id = gen_id();
    let am = retrieval_hits::ActiveModel {
        id: Set(id),
        conversation_id: Set(conversation_id.to_string()),
        message_id: Set(message_id.to_string()),
        knowledge_base_id: Set(knowledge_base_id.to_string()),
        document_id: Set(document_id.to_string()),
        chunk_ref: Set(chunk_ref.to_string()),
        score: Set(score),
        preview: Set(preview.to_string()),
    };
    am.insert(db).await?;
    Ok(())
}

/// Record multiple retrieval hits in bulk.
pub async fn record_hits(
    db: &DatabaseConnection,
    conversation_id: &str,
    message_id: &str,
    hits: &[(String, String, String, f64, String)], // (kb_id, doc_id, chunk_ref, score, preview)
) -> Result<(), DbErr> {
    for (kb_id, doc_id, chunk_ref, score, preview) in hits {
        let _ = record_hit(db, conversation_id, message_id, kb_id, doc_id, chunk_ref, *score, preview).await;
    }
    Ok(())
}
