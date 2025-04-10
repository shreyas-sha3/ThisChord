use sqlx::{PgPool, Error};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}

// Get or create a conversation between two users
pub async fn get_or_create_conversation(
    pool: &PgPool,
    user1: &str,
    user2: &str,
) -> Result<Uuid, Error> {
    let (u1, u2) = if user1 < user2 {
        (user1, user2)
    } else {
        (user2, user1)
    };

    // Use query_scalar! because we're only selecting one value (UUID)
    let existing = sqlx::query_scalar!(
        r#"SELECT id FROM conversations WHERE user1 = $1 AND user2 = $2"#,
        u1,
        u2
    )
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        Ok(id)
    } else {
        let new_id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT INTO conversations (id, user1, user2) VALUES ($1, $2, $3)"#,
            new_id,
            u1,
            u2
        )
        .execute(pool)
        .await?;
        Ok(new_id)
    }
}

// Store a direct message
pub async fn store_direct_message(
    pool: &PgPool,
    conversation_id: Uuid,
    sender: &str,
    content: &str,
) -> Result<(), Error> {
    sqlx::query!(
        r#"
        INSERT INTO messages (conversation_id, sender, content)
        VALUES ($1, $2, $3)
        "#,
        conversation_id,
        sender,
        content
    )
    .execute(pool)
    .await?;
    Ok(())
}

// Load recent direct messages (latest N, returned in correct order)
pub async fn load_direct_messages(
    pool: &PgPool,
    conversation_id: Uuid,
    limit: i64,
) -> Result<Vec<ChatMessage>, Error> {
    let rows = sqlx::query_as!(
        ChatMessage,
        r#"
        SELECT sender, content, sent_at
        FROM messages
        WHERE conversation_id = $1
        ORDER BY sent_at DESC
        LIMIT $2
        "#,
        conversation_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    // Reverse so newest messages appear last
    Ok(rows.into_iter().rev().collect())
}

pub async fn fetch_dm_list(
    pool: &PgPool,
    current_user: &str,
) -> Result<Vec<String>, Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            CASE 
                WHEN user1 = $1 THEN user2 
                ELSE user1 
            END AS dm_partner
        FROM conversations
        WHERE user1 = $1 OR user2 = $1
        "#,
        current_user
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| row.dm_partner) // extract Option<String>
        .collect())
}
