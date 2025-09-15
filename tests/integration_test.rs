// ArbitrageX Supreme V3.0 - Integration Tests
// Comprehensive integration testing for all ArbitrageX services

use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

// Test configuration
const API_BASE_URL: &str = "http://localhost:8080";
const SELECTOR_API_URL: &str = "http://localhost:8081";
const TIMEOUT_SECONDS: u64 = 30;

#[tokio::test]
async fn test_api_server_health() -> Result<()> {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/health", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let health: Value = response.json().await?;
    assert_eq!(health["success"], true);
    assert!(health["data"]["status"] == "healthy" || health["data"]["status"] == "degraded");
    
    println!("✅ API Server health check passed");
    Ok(())
}

#[tokio::test]
async fn test_selector_api_health() -> Result<()> {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/health", SELECTOR_API_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let health: Value = response.json().await?;
    assert_eq!(health["success"], true);
    
    println!("✅ Selector API health check passed");
    Ok(())
}

#[tokio::test]
async fn test_opportunities_endpoint() -> Result<()> {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/api/v1/opportunities", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let opportunities: Value = response.json().await?;
    assert_eq!(opportunities["success"], true);
    assert!(opportunities["data"].is_array());
    
    println!("✅ Opportunities endpoint test passed");
    Ok(())
}

#[tokio::test]
async fn test_executions_endpoint() -> Result<()> {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/api/v1/executions", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let executions: Value = response.json().await?;
    assert_eq!(executions["success"], true);
    assert!(executions["data"].is_array());
    
    println!("✅ Executions endpoint test passed");
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint() -> Result<()> {
    let client = Client::new();
    
    let response = client
        .get(&format!("{}/api/v1/metrics", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let metrics: Value = response.json().await?;
    assert_eq!(metrics["success"], true);
    assert!(metrics["data"]["total_opportunities"].is_number());
    assert!(metrics["data"]["total_executions"].is_number());
    
    println!("✅ Metrics endpoint test passed");
    Ok(())
}

#[tokio::test]
async fn test_authentication_flow() -> Result<()> {
    let client = Client::new();
    
    // Test login
    let login_payload = json!({
        "username": "admin",
        "password": "arbitragex2024"
    });
    
    let response = client
        .post(&format!("{}/api/v1/auth/login", API_BASE_URL))
        .json(&login_payload)
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let auth_response: Value = response.json().await?;
    assert_eq!(auth_response["success"], true);
    assert!(auth_response["data"]["token"].is_string());
    
    println!("✅ Authentication flow test passed");
    Ok(())
}

#[tokio::test]
async fn test_opportunities_filtering() -> Result<()> {
    let client = Client::new();
    
    // Test with chain filter
    let response = client
        .get(&format!("{}/api/v1/opportunities?chain=ethereum&limit=10", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let opportunities: Value = response.json().await?;
    assert_eq!(opportunities["success"], true);
    
    // Test with profit filter
    let response = client
        .get(&format!("{}/api/v1/opportunities?min_profit=100&limit=5", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    println!("✅ Opportunities filtering test passed");
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let client = Client::new();
    
    // Test non-existent endpoint
    let response = client
        .get(&format!("{}/api/v1/nonexistent", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert_eq!(response.status(), 404);
    
    // Test invalid authentication
    let invalid_login = json!({
        "username": "invalid",
        "password": "invalid"
    });
    
    let response = client
        .post(&format!("{}/api/v1/auth/login", API_BASE_URL))
        .json(&invalid_login)
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    let auth_response: Value = response.json().await?;
    assert_eq!(auth_response["success"], false);
    
    println!("✅ Error handling test passed");
    Ok(())
}

#[tokio::test]
async fn test_service_integration() -> Result<()> {
    let client = Client::new();
    
    // Wait for services to be ready
    sleep(Duration::from_secs(5)).await;
    
    // Test that all services are responding
    let services = vec![
        ("API Server", format!("{}/health", API_BASE_URL)),
        ("Selector API", format!("{}/health", SELECTOR_API_URL)),
    ];
    
    for (service_name, url) in services {
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(TIMEOUT_SECONDS))
            .send()
            .await?;
        
        assert!(response.status().is_success(), "Service {} is not responding", service_name);
        println!("✅ {} is responding", service_name);
    }
    
    println!("✅ Service integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_database_connectivity() -> Result<()> {
    let client = Client::new();
    
    // Test that database is connected through API
    let response = client
        .get(&format!("{}/health", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let health: Value = response.json().await?;
    assert_eq!(health["data"]["database_connected"], true);
    
    println!("✅ Database connectivity test passed");
    Ok(())
}

#[tokio::test]
async fn test_redis_connectivity() -> Result<()> {
    let client = Client::new();
    
    // Test that Redis is connected through API
    let response = client
        .get(&format!("{}/health", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    assert!(response.status().is_success());
    
    let health: Value = response.json().await?;
    assert_eq!(health["data"]["redis_connected"], true);
    
    println!("✅ Redis connectivity test passed");
    Ok(())
}

#[tokio::test]
async fn test_performance_benchmarks() -> Result<()> {
    let client = Client::new();
    
    // Test response times
    let start = std::time::Instant::now();
    
    let response = client
        .get(&format!("{}/api/v1/opportunities?limit=100", API_BASE_URL))
        .timeout(Duration::from_secs(TIMEOUT_SECONDS))
        .send()
        .await?;
    
    let duration = start.elapsed();
    
    assert!(response.status().is_success());
    assert!(duration < Duration::from_millis(1000), "Response time too slow: {:?}", duration);
    
    println!("✅ Performance benchmark test passed ({}ms)", duration.as_millis());
    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let client = Client::new();
    
    // Test concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let response = client
                .get(&format!("{}/api/v1/opportunities?limit=10&offset={}", API_BASE_URL, i * 10))
                .timeout(Duration::from_secs(TIMEOUT_SECONDS))
                .send()
                .await?;
            
            assert!(response.status().is_success());
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        handle.await??;
    }
    
    println!("✅ Concurrent requests test passed");
    Ok(())
}

// Helper function to run all tests
pub async fn run_integration_tests() -> Result<()> {
    println!("🚀 Starting ArbitrageX Supreme V3.0 Integration Tests");
    
    // Wait for services to start
    sleep(Duration::from_secs(10)).await;
    
    // Run all tests
    test_api_server_health().await?;
    test_selector_api_health().await?;
    test_opportunities_endpoint().await?;
    test_executions_endpoint().await?;
    test_metrics_endpoint().await?;
    test_authentication_flow().await?;
    test_opportunities_filtering().await?;
    test_error_handling().await?;
    test_service_integration().await?;
    test_database_connectivity().await?;
    test_redis_connectivity().await?;
    test_performance_benchmarks().await?;
    test_concurrent_requests().await?;
    
    println!("🎉 All integration tests passed successfully!");
    Ok(())
}
