// Good ETL code - clean separation of concerns
// Handles the SAME business logic as etl_bad.rs but with proper structure:
// - Test email prefixes, .gov/.mil validation
// - COPPA compliance, minor approval workflows
// - VIP user detection, name truncation
// All separated into pure, testable functions.

use std::error::Error;

#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Clone)]
struct RawUserData {
    id: i64,
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug)]
struct ProcessedUserData {
    id: i64,
    name: String,
    email: String,
    age: u32,
}

#[derive(Debug, Clone)]
enum UserType {
    VipSenior,
    VipPremium,
    Minor,
    RegularAdult,
}

#[derive(Debug, Clone)]
enum EmailType {
    Test,
    Government,
    Educational,
    Regular,
}

// Pure functions for email validation and transformation
fn classify_email(email: &str) -> EmailType {
    if email.starts_with("test_") {
        EmailType::Test
    } else if email.ends_with(".gov") || email.ends_with(".mil") {
        EmailType::Government
    } else if email.ends_with(".edu") {
        EmailType::Educational
    } else {
        EmailType::Regular
    }
}

fn transform_email_by_type(email: String, email_type: &EmailType) -> String {
    match email_type {
        EmailType::Test => email.replace("test_", "verified_test_"),
        EmailType::Government => format!("verified_{}", email),
        _ => email,
    }
}

fn validate_basic_email(email: &str) -> Result<(), Box<dyn Error>> {
    if email.is_empty() {
        return Err("Invalid email".into());
    }
    if !email.contains('@') {
        return Err("Email missing @".into());
    }
    if email.len() > 100 {
        return Err("Email too long".into());
    }
    Ok(())
}

fn normalize_email(email: String) -> String {
    email.to_lowercase().trim().to_string()
}

// Pure functions for age validation
fn validate_coppa_compliance(age: u32) -> Result<(), Box<dyn Error>> {
    if age < 13 {
        Err("User too young - COPPA violation".into())
    } else {
        Ok(())
    }
}

fn validate_age_bounds(age: u32) -> Result<(), Box<dyn Error>> {
    if age > 120 {
        Err("Invalid age".into())
    } else {
        Ok(())
    }
}

fn validate_government_email_age(age: u32, email_type: &EmailType) -> Result<(), Box<dyn Error>> {
    if matches!(email_type, EmailType::Government) && age < 21 {
        Err("Government email requires age 21+".into())
    } else {
        Ok(())
    }
}

fn validate_minor_educational_email(
    age: u32,
    email_type: &EmailType,
) -> Result<(), Box<dyn Error>> {
    if age < 18 && matches!(email_type, EmailType::Educational) {
        Err("Minor with .edu email needs manual approval".into())
    } else {
        Ok(())
    }
}

fn classify_user_by_age(age: u32, email: &str) -> UserType {
    if age >= 65 {
        UserType::VipSenior
    } else if email.contains("premium") || email.contains("vip") {
        UserType::VipPremium
    } else if age < 18 {
        UserType::Minor
    } else {
        UserType::RegularAdult
    }
}

// Pure functions for name validation and transformation
fn validate_name_basic(name: &str) -> Result<(), Box<dyn Error>> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.len() < 2 {
        return Err("Name too short".into());
    }
    Ok(())
}

fn truncate_name_if_needed(name: String) -> (String, bool) {
    if name.len() > 50 {
        (name[..50].to_string(), true)
    } else {
        (name, false)
    }
}

fn capitalize_word(word: &str) -> String {
    if let Some(first) = word.chars().next() {
        first.to_uppercase().to_string() + &word[1..]
    } else {
        String::new()
    }
}

fn capitalize_name(name: String) -> String {
    if name.contains(' ') {
        // Multiple words - capitalize each
        name.split_whitespace()
            .map(capitalize_word)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        // Single word
        capitalize_word(&name)
    }
}

fn normalize_name(name: String) -> Result<String, Box<dyn Error>> {
    let trimmed = name.trim().to_string();
    validate_name_basic(&trimmed)?;
    let (truncated, _was_truncated) = truncate_name_if_needed(trimmed);
    Ok(capitalize_name(truncated))
}

// Pure transformation pipeline - composes all the pure functions
fn transform_user_data(raw: RawUserData) -> Result<ProcessedUserData, Box<dyn Error>> {
    // Email validation and transformation
    validate_basic_email(&raw.email)?;
    let email_type = classify_email(&raw.email);
    validate_government_email_age(raw.age, &email_type)?;
    validate_minor_educational_email(raw.age, &email_type)?;

    let transformed_email = transform_email_by_type(raw.email, &email_type);
    let normalized_email = normalize_email(transformed_email);

    // Age validation
    validate_coppa_compliance(raw.age)?;
    validate_age_bounds(raw.age)?;

    // Name transformation
    let normalized_name = normalize_name(raw.name)?;

    Ok(ProcessedUserData {
        id: raw.id,
        name: normalized_name,
        email: normalized_email,
        age: raw.age,
    })
}

// I/O infrastructure layer
struct Database;
struct EventBus;

impl Database {
    async fn fetch_user(&self, id: i64) -> Result<RawUserData, Box<dyn Error>> {
        Ok(RawUserData {
            id,
            name: "  test user  ".to_string(),
            email: "TEST@EXAMPLE.COM".to_string(),
            age: 30,
        })
    }

    async fn save(&self, data: &ProcessedUserData) -> Result<(), Box<dyn Error>> {
        println!("Saving user: {:?}", data);
        Ok(())
    }
}

impl EventBus {
    async fn publish(&self, event: &str, user_id: i64) -> Result<(), Box<dyn Error>> {
        println!("Publishing event: {} for user {}", event, user_id);
        Ok(())
    }
}

// Determine events to publish based on user classification
fn get_events_for_user(user_type: &UserType) -> Vec<&'static str> {
    match user_type {
        UserType::VipSenior => vec!["user.vip.senior"],
        UserType::VipPremium => vec!["user.vip.premium"],
        UserType::Minor => vec![],
        UserType::RegularAdult => vec![],
    }
}

fn should_warn_minor(age: u32) -> bool {
    age >= 13 && age < 18
}

fn determine_young_adult_event(age: u32) -> Option<&'static str> {
    if age >= 18 && age < 25 {
        Some("user.young_adult")
    } else {
        None
    }
}

// I/O at the edges - infrastructure concerns only
// This function orchestrates I/O but delegates business logic to pure functions
async fn process_user_data(
    user_id: i64,
    db: &Database,
    event_bus: &EventBus,
) -> Result<(), Box<dyn Error>> {
    let raw = db.fetch_user(user_id).await?;

    // Pure transformation - all business logic here
    let processed = transform_user_data(raw.clone())?;

    // Classify user for event publishing
    let user_type = classify_user_by_age(processed.age, &processed.email);

    // Side effects (logging) - kept at I/O boundary
    if should_warn_minor(processed.age) {
        println!("Warning: Minor user requires parental consent");
    }

    println!(
        "Processing user: {} (age: {}, email: {})",
        processed.id, processed.age, processed.email
    );

    if matches!(user_type, UserType::Minor) {
        println!("Saving to minors table");
    }

    // I/O operations
    db.save(&processed).await?;
    event_bus.publish("user.updated", user_id).await?;

    // Publish additional events based on classification
    for event in get_events_for_user(&user_type) {
        event_bus.publish(event, user_id).await?;
    }

    if let Some(event) = determine_young_adult_event(processed.age) {
        event_bus.publish(event, user_id).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = Database;
    let event_bus = EventBus;

    process_user_data(1, &db, &event_bus).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_email() {
        assert!(matches!(
            classify_email("test_user@example.com"),
            EmailType::Test
        ));
        assert!(matches!(
            classify_email("user@agency.gov"),
            EmailType::Government
        ));
        assert!(matches!(
            classify_email("student@university.edu"),
            EmailType::Educational
        ));
        assert!(matches!(
            classify_email("user@example.com"),
            EmailType::Regular
        ));
    }

    #[test]
    fn test_transform_email_by_type() {
        assert_eq!(
            transform_email_by_type("test_user@example.com".to_string(), &EmailType::Test),
            "verified_test_user@example.com"
        );
        assert_eq!(
            transform_email_by_type("user@gov.com".to_string(), &EmailType::Government),
            "verified_user@gov.com"
        );
    }

    #[test]
    fn test_validate_coppa_compliance() {
        assert!(validate_coppa_compliance(12).is_err());
        assert!(validate_coppa_compliance(13).is_ok());
        assert!(validate_coppa_compliance(18).is_ok());
    }

    #[test]
    fn test_validate_government_email_age() {
        assert!(validate_government_email_age(20, &EmailType::Government).is_err());
        assert!(validate_government_email_age(21, &EmailType::Government).is_ok());
        assert!(validate_government_email_age(18, &EmailType::Regular).is_ok());
    }

    #[test]
    fn test_validate_minor_educational_email() {
        assert!(validate_minor_educational_email(17, &EmailType::Educational).is_err());
        assert!(validate_minor_educational_email(18, &EmailType::Educational).is_ok());
        assert!(validate_minor_educational_email(17, &EmailType::Regular).is_ok());
    }

    #[test]
    fn test_classify_user_by_age() {
        assert!(matches!(
            classify_user_by_age(70, "user@example.com"),
            UserType::VipSenior
        ));
        assert!(matches!(
            classify_user_by_age(30, "premium@example.com"),
            UserType::VipPremium
        ));
        assert!(matches!(
            classify_user_by_age(16, "user@example.com"),
            UserType::Minor
        ));
        assert!(matches!(
            classify_user_by_age(25, "user@example.com"),
            UserType::RegularAdult
        ));
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("  john  ".to_string()).unwrap(), "John");
        assert_eq!(
            normalize_name("john doe".to_string()).unwrap(),
            "John Doe"
        );
        assert!(normalize_name("a".to_string()).is_err()); // too short
        assert!(normalize_name("".to_string()).is_err()); // empty
    }

    #[test]
    fn test_truncate_name_if_needed() {
        let long_name = "a".repeat(60);
        let (truncated, was_truncated) = truncate_name_if_needed(long_name);
        assert_eq!(truncated.len(), 50);
        assert!(was_truncated);

        let short_name = "John Doe".to_string();
        let (result, was_truncated) = truncate_name_if_needed(short_name.clone());
        assert_eq!(result, short_name);
        assert!(!was_truncated);
    }

    #[test]
    fn test_transform_user_data() {
        let raw = RawUserData {
            id: 1,
            name: "  test user  ".to_string(),
            email: "TEST@EXAMPLE.COM".to_string(),
            age: 25,
        };

        let result = transform_user_data(raw).unwrap();
        assert_eq!(result.email, "test@example.com");
        assert_eq!(result.name, "Test User");
        assert_eq!(result.age, 25);
    }

    #[test]
    fn test_transform_user_data_government_email_too_young() {
        let raw = RawUserData {
            id: 1,
            name: "Test User".to_string(),
            email: "user@agency.gov".to_string(),
            age: 20,
        };

        assert!(transform_user_data(raw).is_err());
    }

    #[test]
    fn test_transform_user_data_minor_edu_email() {
        let raw = RawUserData {
            id: 1,
            name: "Test User".to_string(),
            email: "student@university.edu".to_string(),
            age: 17,
        };

        assert!(transform_user_data(raw).is_err());
    }
}
