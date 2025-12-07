// Bad event handler - everything tangled together
// Event infrastructure, business logic, and side effects all mixed.
// Requires understanding the entire system to modify any part.

use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    topic: String,
    payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserRegistered {
    id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmailSent {
    user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrialStarted {
    user_id: i64,
    expires_at: String,
}

#[derive(Debug)]
struct User {
    id: i64,
    email: String,
    email_verified: bool,
    plan_id: i64,
}

#[derive(Debug)]
struct Plan {
    id: i64,
    name: String,
    is_trial: bool,
}

struct Database;
struct EmailService;
struct EventBus;
struct Analytics;

impl Database {
    async fn get_user(&self, id: i64) -> Result<User, Box<dyn Error>> {
        Ok(User {
            id,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        })
    }

    async fn get_plan(&self, id: i64) -> Result<Plan, Box<dyn Error>> {
        Ok(Plan {
            id,
            name: "Trial Plan".to_string(),
            is_trial: true,
        })
    }
}

impl EmailService {
    async fn send(&self, _email: String) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl EventBus {
    async fn publish(&self, _event: Event) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl Analytics {
    async fn track(&self, _event: &str, _user_id: i64) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

fn format_welcome_email(_user: &User, _plan: &Plan) -> String {
    "Welcome!".to_string()
}

// Event handler with infrastructure, business logic, and side effects mixed
// High cognitive complexity - needs to understand event bus, email service,
// analytics, database schema, trial logic, promotional logic, notification preferences,
// referral tracking, and all downstream handlers
async fn on_user_registered(
    event: Event,
    db: &Database,
    email_service: &EmailService,
    event_bus: &EventBus,
    analytics: &Analytics,
) -> Result<(), Box<dyn Error>> {
    // Deserialize event payload with error handling mixed in
    let user_data: UserRegistered = serde_json::from_str(&event.payload)?;

    // Fetch additional data with nested queries
    let user = db.get_user(user_data.id).await?;
    let plan = db.get_plan(user.plan_id).await?;

    // Complex email verification logic
    if user.email_verified {
        // Nested conditions for different email types
        if user.email.ends_with(".edu") {
            // Educational users get special onboarding
            let welcome_email = format_welcome_email(&user, &plan);
            email_service.send(welcome_email).await?;

            event_bus
                .publish(Event {
                    topic: "email.sent.educational".to_string(),
                    payload: serde_json::to_string(&EmailSent {
                        user_id: user.id,
                    })?,
                })
                .await?;

            analytics.track("educational_user_registered", user.id).await?;
        } else if user.email.ends_with(".gov") || user.email.ends_with(".mil") {
            // Government users need approval workflow
            event_bus
                .publish(Event {
                    topic: "user.needs_approval".to_string(),
                    payload: serde_json::to_string(&UserRegistered {
                        id: user.id,
                    })?,
                })
                .await?;

            analytics.track("government_user_registered", user.id).await?;
        } else {
            // Regular users
            let welcome_email = format_welcome_email(&user, &plan);
            email_service.send(welcome_email).await?;

            // Publish email sent event
            event_bus
                .publish(Event {
                    topic: "email.sent".to_string(),
                    payload: serde_json::to_string(&EmailSent {
                        user_id: user.id,
                    })?,
                })
                .await?;
        }
    } else {
        // Send verification email
        event_bus
            .publish(Event {
                topic: "email.verification_needed".to_string(),
                payload: serde_json::to_string(&UserRegistered {
                    id: user.id,
                })?,
            })
            .await?;
    }

    // Track registration with complex conditional logic
    if plan.is_trial {
        analytics.track("trial_user_registered", user.id).await?;

        // Trial users get special onboarding sequence
        event_bus
            .publish(Event {
                topic: "trial.started".to_string(),
                payload: serde_json::to_string(&TrialStarted {
                    user_id: user.id,
                    expires_at: "2024-01-01".to_string(),
                })?,
            })
            .await?;

        // Check if promotional trial
        if plan.name.contains("promo") || plan.name.contains("discount") {
            analytics.track("promo_trial_started", user.id).await?;

            event_bus
                .publish(Event {
                    topic: "marketing.promo_conversion".to_string(),
                    payload: serde_json::to_string(&UserRegistered {
                        id: user.id,
                    })?,
                })
                .await?;
        }
    } else {
        analytics.track("paid_user_registered", user.id).await?;

        // Paid users get different onboarding
        if plan.name.contains("premium") || plan.name.contains("enterprise") {
            event_bus
                .publish(Event {
                    topic: "user.premium_registered".to_string(),
                    payload: serde_json::to_string(&UserRegistered {
                        id: user.id,
                    })?,
                })
                .await?;

            analytics.track("premium_user_registered", user.id).await?;
        }
    }

    // Check for referral tracking (more nested logic)
    if event.payload.contains("referral_code") {
        analytics.track("referred_user_registered", user.id).await?;

        event_bus
            .publish(Event {
                topic: "referral.conversion".to_string(),
                payload: event.payload.clone(),
            })
            .await?;

        // Give referrer credit
        if user.email_verified && !plan.is_trial {
            event_bus
                .publish(Event {
                    topic: "referral.credit_earned".to_string(),
                    payload: event.payload.clone(),
                })
                .await?;
        }
    }

    // Final tracking (always done)
    analytics.track("user_registered", user.id).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = Database;
    let email_service = EmailService;
    let event_bus = EventBus;
    let analytics = Analytics;

    let event = Event {
        topic: "user.registered".to_string(),
        payload: serde_json::to_string(&UserRegistered { id: 1 })?,
    };

    on_user_registered(event, &db, &email_service, &event_bus, &analytics).await?;

    Ok(())
}
