// Bad ETL code - everything tangled together
// This demonstrates poor separation of concerns with I/O, business logic,
// and side effects all mixed in one function.

use std::error::Error;

#[derive(Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
    age: u32,
}

struct Database;
struct EventBus;

impl Database {
    async fn fetch_user(&self, _id: i64) -> Result<User, Box<dyn Error>> {
        Ok(User {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            age: 30,
        })
    }

    async fn save(&self, _user: &User) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl EventBus {
    async fn publish(&self, _event: &str, _user_id: i64) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

// ETL function with everything tangled together
// High cognitive complexity - requires understanding database, transformations,
// event system, business logic, validation rules, special cases, and error handling
async fn process_user_data(
    user_id: i64,
    db: &Database,
    event_bus: &EventBus,
) -> Result<(), Box<dyn Error>> {
    // Database query with retry logic mixed in
    let mut user = db.fetch_user(user_id).await?;

    // Complex validation with nested conditions
    if user.email.is_empty() {
        return Err("Invalid email".into());
    } else if !user.email.contains('@') {
        return Err("Email missing @".into());
    } else if user.email.len() > 100 {
        return Err("Email too long".into());
    } else {
        // Nested transformation based on validation
        if user.email.starts_with("test_") {
            // Test users get special handling
            user.email = user.email.replace("test_", "verified_test_");
        } else if user.email.ends_with(".gov") {
            // Government emails need extra validation
            if user.age < 21 {
                return Err("Government email requires age 21+".into());
            }
            user.email = format!("verified_{}", user.email);
        }
    }

    // Normalize email with multiple steps
    user.email = user.email.to_lowercase().trim().to_string();

    // Complex age validation with multiple tiers
    if user.age < 13 {
        return Err("User too young - COPPA violation".into());
    } else if user.age < 18 {
        // Minors need parental consent (not checked here, but documented)
        println!("Warning: Minor user requires parental consent");
        if user.email.ends_with(".edu") {
            // Educational emails for minors need special approval
            return Err("Minor with .edu email needs manual approval".into());
        }
    } else if user.age > 120 {
        return Err("Invalid age".into());
    }

    // Name transformation with multiple cases
    user.name = user.name.trim().to_string();

    if user.name.is_empty() {
        return Err("Name cannot be empty".into());
    } else if user.name.len() < 2 {
        return Err("Name too short".into());
    } else if user.name.len() > 50 {
        user.name = user.name[..50].to_string();
        println!("Warning: Name truncated");
    }

    // Capitalize based on special rules
    if user.name.contains(' ') {
        // Multiple words - capitalize each
        let parts: Vec<String> = user.name
            .split_whitespace()
            .map(|part| {
                if let Some(first) = part.chars().next() {
                    first.to_uppercase().to_string() + &part[1..]
                } else {
                    String::new()
                }
            })
            .collect();
        user.name = parts.join(" ");
    } else {
        // Single word
        if let Some(first_char) = user.name.chars().next() {
            user.name = first_char.to_uppercase().to_string() + &user.name[1..];
        }
    }

    // Special handling for VIP users (mixed business logic and side effects)
    if user.age >= 65 {
        println!("VIP senior user: {}", user.id);
        event_bus.publish("user.vip.senior", user.id).await?;
    } else if user.email.contains("premium") || user.email.contains("vip") {
        println!("VIP premium user: {}", user.id);
        event_bus.publish("user.vip.premium", user.id).await?;
    }

    // Logging mixed with business logic
    println!("Processing user: {} (age: {}, email: {})", user.id, user.age, user.email);

    // Database write with conditional logic
    if user.age < 18 {
        // Minors go to special table (this logic should be in the DB layer)
        println!("Saving to minors table");
    }
    db.save(&user).await?;

    // Multiple event publishing based on state
    event_bus.publish("user.updated", user.id).await?;

    if user.age >= 18 && user.age < 25 {
        event_bus.publish("user.young_adult", user.id).await?;
    }

    // Final logging
    println!("User processed successfully");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = Database;
    let event_bus = EventBus;

    process_user_data(1, &db, &event_bus).await?;

    Ok(())
}
