//! Integration test for proposal execution API

// This test would verify that:
// 1. A proposal can be transitioned to APPROVED state
// 2. The execute endpoint properly executes the proposal's DSL logic
// 3. The proposal state is updated to EXECUTED
// 4. The execution result is stored

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use warp::test::request;
    use warp::http::StatusCode;
    use serde_json::json;
    use icn_covm::storage::InMemoryStorage;
    use icn_covm::vm::VM;
    use icn_covm::api::v1::proposals;

    // Helper function to set up test environment
    async fn setup_test_proposal() -> (String, Arc<InMemoryStorage>, VM<InMemoryStorage>) {
        // Create storage
        let mut storage = InMemoryStorage::new();
        
        // Create a proposal
        let proposal_id = "test-proposal-123".to_string();
        let proposal = icn_covm::storage::Proposal {
            id: proposal_id.clone(),
            title: "Test Proposal".to_string(),
            description: "This is a test proposal".to_string(),
            status: "DRAFT".to_string(),
            author: "test-user".to_string(),
            created_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            votes_for: Some(10),
            votes_against: Some(2),
            votes_abstain: Some(1),
            attachments: None,
        };
        
        // Save proposal to storage
        storage.save_proposal(&proposal).await.unwrap();
        
        // Create DSL logic for the proposal
        let logic = r#"
        # Simple test logic
        push 42
        store "test_result"
        emitevent "proposal_execution" "Test proposal executed successfully"
        "#;
        
        // Store logic path in metadata
        let metadata = json!({
            "logic_path": format!("proposals/{}/logic", proposal_id)
        });
        
        // Save metadata
        storage.set(
            None, 
            "governance", 
            &format!("proposals/{}/metadata", proposal_id),
            serde_json::to_vec(&metadata).unwrap()
        ).unwrap();
        
        // Save logic
        storage.set(
            None,
            "governance",
            &format!("proposals/{}/logic", proposal_id),
            logic.as_bytes().to_vec()
        ).unwrap();
        
        // Create VM
        let vm = VM::new(storage.clone());
        
        // Return test data
        (proposal_id, Arc::new(storage), vm)
    }
    
    // Approve a proposal - transition it from DRAFT to APPROVED
    async fn approve_proposal(
        storage: &Arc<InMemoryStorage>, 
        proposal_id: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut proposal = storage.get_proposal(proposal_id).await?;
        proposal.status = "APPROVED".to_string();
        storage.clone().save_proposal(&proposal).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_proposal_execution() {
        // Setup test data
        let (proposal_id, storage, vm) = setup_test_proposal().await;
        
        // Approve the proposal
        approve_proposal(&storage, &proposal_id).await.unwrap();
        
        // Create the filter with the execution endpoint
        let routes = proposals::get_routes(storage.clone(), vm);
        
        // Simulate a request to execute the proposal
        let response = request()
            .method("POST")
            .path(&format!("/proposals/{}/execute", proposal_id))
            .header("Authorization", "Bearer test-token")
            .reply(&routes)
            .await;
        
        // Check the response
        assert_eq!(response.status(), StatusCode::OK);
        
        // Parse response body
        let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
        
        // Check that the execution was successful
        assert_eq!(body["status"], "success");
        assert_eq!(body["data"]["status"], "EXECUTED");
        
        // Get the updated proposal
        let proposal = storage.get_proposal(&proposal_id).await.unwrap();
        
        // Verify that the proposal status was updated
        assert_eq!(proposal.status, "EXECUTED");
        
        // Check that the execution result was stored
        let result = storage.get(
            None,
            "governance",
            &format!("proposals/{}/execution_result", proposal_id)
        ).ok().map(|bytes| String::from_utf8(bytes).ok()).flatten();
        
        assert!(result.is_some());
    }
} 