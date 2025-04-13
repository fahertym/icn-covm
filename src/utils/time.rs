use chrono::{DateTime, Duration, TimeZone, Utc};
use std::thread::sleep;
use std::time::Duration as StdDuration;

/// Formats a duration into a human-readable string
/// (e.g., "2h 3m 10s" or "10m 5s")
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    if total_seconds <= 0 {
        return "0s".to_string();
    }

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut result = String::new();
    if hours > 0 {
        result.push_str(&format!("{}h ", hours));
    }
    if minutes > 0 || hours > 0 {
        result.push_str(&format!("{}m ", minutes));
    }
    result.push_str(&format!("{}s", seconds));

    result
}

/// Returns the remaining time until a cooldown expires
pub fn get_cooldown_remaining(last_retry: &DateTime<Utc>, cooldown_duration: Duration) -> Duration {
    let now = Utc::now();
    let expiry = *last_retry + cooldown_duration;
    
    if now >= expiry {
        Duration::seconds(0)
    } else {
        expiry - now
    }
}

/// Displays a countdown timer for the cooldown period
/// Returns immediately if no cooldown is active
pub fn display_cooldown_countdown(last_retry: &DateTime<Utc>, cooldown_duration: Duration) -> bool {
    let remaining = get_cooldown_remaining(last_retry, cooldown_duration);
    if remaining.num_seconds() <= 0 {
        println!("✅ Cooldown period has expired. Retry is now available.");
        return false;
    }

    println!("⏳ Cooldown active. Time remaining before next retry: {}", format_duration(remaining));
    
    // If we're in an interactive session, we could do a live countdown
    // But for now, just return that cooldown is active
    true
}

/// Interactive countdown display that updates in real-time
/// This will block until the countdown completes
/// Set `update_interval_ms` to control how often the display updates
pub fn interactive_countdown(last_retry: &DateTime<Utc>, cooldown_duration: Duration, update_interval_ms: u64) {
    let mut remaining = get_cooldown_remaining(last_retry, cooldown_duration);
    if remaining.num_seconds() <= 0 {
        println!("✅ Cooldown period has expired. Retry is now available.");
        return;
    }
    
    println!("⏳ Cooldown active. Waiting for cooldown to expire...");
    
    // Calculate the end time
    let end_time = Utc::now() + remaining;
    
    // Update the countdown until it reaches zero
    while remaining.num_seconds() > 0 {
        // Clear the line and print the new countdown
        print!("\r⏳ Time remaining: {}    ", format_duration(remaining));
        let _ = std::io::stdout().flush();
        
        // Sleep for the update interval
        sleep(StdDuration::from_millis(update_interval_ms));
        
        // Recalculate remaining time
        remaining = end_time - Utc::now();
    }
    
    println!("\n✅ Cooldown period has expired. Retry is now available.");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::seconds(5)), "5s");
        assert_eq!(format_duration(Duration::seconds(65)), "1m 5s");
        assert_eq!(format_duration(Duration::seconds(3665)), "1h 1m 5s");
        assert_eq!(format_duration(Duration::seconds(0)), "0s");
    }
    
    #[test]
    fn test_get_cooldown_remaining() {
        let now = Utc::now();
        let cooldown = Duration::minutes(30);
        
        // Test with a retry time in the past (cooldown expired)
        let past_retry = now - Duration::hours(1);
        assert_eq!(get_cooldown_remaining(&past_retry, cooldown).num_seconds(), 0);
        
        // Test with a recent retry (cooldown active)
        let recent_retry = now - Duration::minutes(15);
        let remaining = get_cooldown_remaining(&recent_retry, cooldown);
        assert!(remaining.num_minutes() <= 15);
        assert!(remaining.num_minutes() >= 14);
    }
} 