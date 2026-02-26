use std::path::PathBuf;

#[test]
fn test_config_preserves_unknown_fields() {
    use crate::Config;
    
    // Load actual config
    let config = Config::load().unwrap();
    
    // Check that mcp field was captured in extra
    assert!(!config.extra.is_empty(), "extra should contain unknown fields");
    assert!(config.extra.contains_key("mcp"), "extra should contain 'mcp'");
    
    println!("✓ Config successfully captured 'mcp' in extra field");
    
    // Now serialize back
    let json = serde_json::to_string_pretty(&config).unwrap();
    
    // Verify mcp is still there
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("mcp").is_some(), "Serialized config should still have 'mcp'");
    
    println!("✓ Serialized config still contains 'mcp'");
}
