use super::*;
use std::path::PathBuf;
use tokio::fs;
use std::time::Duration;

#[tokio::test]
async fn test_atomic_session_save() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path().join("session.json").to_str().unwrap().to_string();
    
    let session = Session::new(
        "test-session".to_string(),
        vec![],
        RepoRef::default(),
        storage_path.clone(),
        UserContext::default(),
        vec![],
    );

    // Save should succeed
    session.save_to_storage().await.unwrap();
    
    // Verify file exists and temp file doesn't
    assert!(fs::metadata(&storage_path).await.is_ok());
    assert!(fs::metadata(format!("{}.tmp", storage_path)).await.is_err());
    
    // Content should be valid JSON
    let content = fs::read_to_string(&storage_path).await.unwrap();
    serde_json::from_str::<Session>(&content).unwrap();
}

#[tokio::test]
async fn test_concurrent_session_saves() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path().join("session.json").to_str().unwrap().to_string();
    
    let session1 = Session::new(
        "test-session-1".to_string(),
        vec![],
        RepoRef::default(),
        storage_path.clone(),
        UserContext::default(),
        vec![],
    );

    let session2 = Session::new(
        "test-session-2".to_string(),
        vec![],
        RepoRef::default(),
        storage_path.clone(),
        UserContext::default(),
        vec![],
    );

    // Start concurrent saves
    let save1 = tokio::spawn(async move { session1.save_to_storage().await });
    let save2 = tokio::spawn(async move { session2.save_to_storage().await });
    
    // Both should complete without errors
    let (result1, result2) = tokio::join!(save1, save2);
    result1.unwrap().unwrap();
    result2.unwrap().unwrap();
    
    // File should exist and be valid JSON
    let content = fs::read_to_string(&storage_path).await.unwrap();
    serde_json::from_str::<Session>(&content).unwrap();
}

#[tokio::test]
async fn test_session_save_with_interruption() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path().join("session.json").to_str().unwrap().to_string();
    
    let session = Session::new(
        "test-session".to_string(),
        vec![],
        RepoRef::default(),
        storage_path.clone(),
        UserContext::default(),
        vec![],
    );

    // Create initial file
    session.save_to_storage().await.unwrap();
    let original_content = fs::read_to_string(&storage_path).await.unwrap();
    
    // Simulate interrupted write by creating .tmp file
    fs::write(format!("{}.tmp", storage_path), "corrupted").await.unwrap();
    
    // Save should still succeed
    session.save_to_storage().await.unwrap();
    
    // Temp file should be gone and content should be valid
    assert!(fs::metadata(format!("{}.tmp", storage_path)).await.is_err());
    let new_content = fs::read_to_string(&storage_path).await.unwrap();
    assert_ne!(new_content, "corrupted");
    serde_json::from_str::<Session>(&new_content).unwrap();
}

#[tokio::test]
async fn test_session_load_with_trailing_chars() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage_path = temp_dir.path().join("session.json").to_str().unwrap().to_string();
    
    let session = Session::new(
        "test-session".to_string(),
        vec![],
        RepoRef::default(),
        storage_path.clone(),
        UserContext::default(),
        vec![],
    );

    // Save initial content
    session.save_to_storage().await.unwrap();
    
    // Add trailing characters
    let mut content = fs::read_to_string(&storage_path).await.unwrap();
    content.push_str("\n\n\r\n");
    fs::write(&storage_path, content).await.unwrap();
    
    // Service should still be able to load
    let service = SessionService::new(
        Arc::new(ToolBox::default()),
        Arc::new(SymbolManager::default()),
    );
    
    let loaded = service.load_from_storage(storage_path).await;
    assert!(loaded.is_ok());
}