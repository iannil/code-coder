use cron::Schedule;
use std::str::FromStr;

#[test]
fn test_cron_expressions() {
    // Test scan_cron: "0 18 * * 1-5" -> "0 0 18 * * 1-5"
    let scan_exp = "0 18 * * 1-5";
    let scan_with_sec = format!("0 {}", scan_exp);
    println!("Testing scan: '{}' -> '{}'", scan_exp, scan_with_sec);
    assert!(Schedule::from_str(&scan_with_sec).is_ok(), "Scan expression should be valid");
    
    // Test sync_cron with 7 (Sunday in cron crate): "0 20 * * 7" -> "0 0 20 * * 7"
    // Note: cron crate uses 1-7 for days (1=Sunday), not 0-6 like traditional cron
    let sync_exp = "0 20 * * 7";
    let sync_with_sec = format!("0 {}", sync_exp);
    println!("Testing sync: '{}' -> '{}'", sync_exp, sync_with_sec);
    let result = Schedule::from_str(&sync_with_sec);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok(), "Sync expression should be valid");
    
    // Test with SUN (day name)
    let sync_sun_exp = "0 20 * * SUN";
    let sync_sun_with_sec = format!("0 {}", sync_sun_exp);
    println!("Testing sync with SUN: '{}' -> '{}'", sync_sun_exp, sync_sun_with_sec);
    assert!(Schedule::from_str(&sync_sun_with_sec).is_ok(), "Sync with SUN should be valid");
}
