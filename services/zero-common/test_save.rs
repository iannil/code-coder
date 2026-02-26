use serde_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string("/Users/iannil/.codecoder/config.json")?;
    
    // Parse original
    let original: serde_json::Value = serde_json::from_str(&content)?;
    
    // Count original keys
    let original_count = original.as_object().map(|o| o.len()).unwrap_or(0);
    println!("Original config.json has {} keys", original_count);
    
    // Check if it has 'mcp'
    let has_mcp = original.get("mcp").is_some();
    println!("Original has 'mcp': {}", has_mcp);
    
    // Now simulate what Config::save() would do
    // It would serialize the Config struct which doesn't have 'mcp'
    let mut without_mcp = original.clone();
    without_mcp.as_object_mut().unwrap().remove("mcp");
    
    let new_count = without_mcp.as_object().map(|o| o.len()).unwrap_or(0);
    println!("After Config::save() would have {} keys", new_count);
    println!("Lost keys: {}", original_count - new_count);
    
    // Show what would be lost
    if let Some(obj) = original.as_object() {
        for key in obj.keys() {
            if without_mcp.get(key).is_none() {
                println!("  - '{}' would be lost", key);
            }
        }
    }
    
    Ok(())
}
