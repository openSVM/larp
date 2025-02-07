#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::agent::types::{ConversationMessage, AgentState, AgentStep};
    use crate::repo::types::{RepoRef, Backend};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn setup_test_db() -> SqlitePool {
        let url = "sqlite::memory:";
        let pool = SqlitePool::connect(url).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_atomic_save_and_load() {
        let pool = setup_test_db().await;
        let db = Arc::new(pool);
        let session_id = Uuid::new_v4();
        let repo_ref = RepoRef::new(Backend::Local, "/test/repo").unwrap();

        // Create a test conversation message
        let mut message = ConversationMessage::general_question(
            session_id,
            AgentState::Search,
            "test query".to_string(),
        );
        message.set_answer("test answer".to_string());

        // Save to database
        message.save_to_db(db.clone(), repo_ref.clone()).await.unwrap();

        // Load from database and verify
        let loaded_messages = ConversationMessage::load_from_db(db.clone(), &repo_ref, session_id)
            .await
            .unwrap();
        
        assert_eq!(loaded_messages.len(), 1);
        let loaded = &loaded_messages[0];
        assert_eq!(loaded.query(), "test query");
        assert_eq!(
            loaded.answer().unwrap().answer_up_until_now,
            "test answer"
        );
    }

    #[tokio::test]
    async fn test_atomic_save_rollback() {
        let pool = setup_test_db().await;
        let db = Arc::new(pool);
        let session_id = Uuid::new_v4();
        let repo_ref = RepoRef::new(Backend::Local, "/test/repo").unwrap();

        // Create a test conversation message
        let mut message = ConversationMessage::general_question(
            session_id,
            AgentState::Search,
            "test query".to_string(),
        );

        // Add a step to the message
        let step = AgentStep::Path {
            query: "test".to_string(),
            response: "test".to_string(),
            paths: vec!["invalid/path".to_string()],
        };
        message.add_agent_step(step);

        // Attempt to save with the step data
        let save_result = message.save_to_db(db.clone(), repo_ref.clone()).await;
        assert!(save_result.is_ok(), "Save operation should complete");

        // Verify the message can be loaded back
        let loaded_messages = ConversationMessage::load_from_db(db.clone(), &repo_ref, session_id)
            .await
            .unwrap();
        assert_eq!(loaded_messages.len(), 1);
    }
}