// Good event handler - clean separation of concerns
// Handles the SAME business logic as events_bad.rs but with proper structure:
// - Educational/government/military email handling
// - Trial vs paid logic, promotional trials
// - Referral tracking and credit
// All separated into pure domain logic with no infrastructure concerns.

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

#[derive(Debug)]
struct User {
    id: i64,
    email: String,
    email_verified: bool,
    plan_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlanType {
    Trial,
    Paid,
}

#[derive(Debug)]
struct Plan {
    id: i64,
    name: String,
    plan_type: PlanType,
    is_trial: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EmailDomain {
    Educational,
    Government,
    Regular,
}

#[derive(Debug, Clone, Copy)]
enum PlanTier {
    Premium,
    Enterprise,
    Standard,
}

// Domain events - pure data, no infrastructure
#[derive(Debug, Clone, PartialEq)]
enum DomainEvent {
    SendWelcomeEmail { user_id: i64, email_domain: EmailDomain },
    SendVerificationEmail { user_id: i64 },
    RequestApproval { user_id: i64, reason: &'static str },
    TrackRegistration { user_id: i64, event_name: &'static str },
    StartTrial { user_id: i64, expires_at: String },
    NotifyPremiumSignup { user_id: i64 },
    TrackPromoConversion { user_id: i64 },
    TrackReferralConversion { user_id: i64 },
    AwardReferralCredit { user_id: i64 },
}

// Pure classification functions
fn classify_email_domain(email: &str) -> EmailDomain {
    if email.ends_with(".edu") {
        EmailDomain::Educational
    } else if email.ends_with(".gov") || email.ends_with(".mil") {
        EmailDomain::Government
    } else {
        EmailDomain::Regular
    }
}

fn classify_plan_tier(plan_name: &str) -> PlanTier {
    if plan_name.contains("premium") || plan_name.contains("Premium") {
        PlanTier::Premium
    } else if plan_name.contains("enterprise") || plan_name.contains("Enterprise") {
        PlanTier::Enterprise
    } else {
        PlanTier::Standard
    }
}

fn is_promotional_plan(plan_name: &str) -> bool {
    plan_name.contains("promo") || plan_name.contains("discount")
}

fn has_referral_code(payload: &str) -> bool {
    payload.contains("referral_code")
}

// Pure business logic - determine events based on email verification
fn handle_email_verification(
    user: &User,
    email_domain: EmailDomain,
) -> Vec<DomainEvent> {
    let mut events = vec![];

    if user.email_verified {
        // Send appropriate welcome email
        events.push(DomainEvent::SendWelcomeEmail {
            user_id: user.id,
            email_domain,
        });

        // Handle special email domains
        match email_domain {
            EmailDomain::Educational => {
                events.push(DomainEvent::TrackRegistration {
                    user_id: user.id,
                    event_name: "educational_user_registered",
                });
            }
            EmailDomain::Government => {
                events.push(DomainEvent::RequestApproval {
                    user_id: user.id,
                    reason: "government_email",
                });
                events.push(DomainEvent::TrackRegistration {
                    user_id: user.id,
                    event_name: "government_user_registered",
                });
            }
            EmailDomain::Regular => {}
        }
    } else {
        events.push(DomainEvent::SendVerificationEmail {
            user_id: user.id,
        });
    }

    events
}

// Pure business logic - determine events based on plan
fn handle_plan_events(user: &User, plan: &Plan) -> Vec<DomainEvent> {
    let mut events = vec![];

    if plan.is_trial {
        events.push(DomainEvent::TrackRegistration {
            user_id: user.id,
            event_name: "trial_user_registered",
        });

        events.push(DomainEvent::StartTrial {
            user_id: user.id,
            expires_at: "2024-01-01".to_string(),
        });

        // Check for promotional trial
        if is_promotional_plan(&plan.name) {
            events.push(DomainEvent::TrackRegistration {
                user_id: user.id,
                event_name: "promo_trial_started",
            });
            events.push(DomainEvent::TrackPromoConversion {
                user_id: user.id,
            });
        }
    } else {
        events.push(DomainEvent::TrackRegistration {
            user_id: user.id,
            event_name: "paid_user_registered",
        });

        // Handle premium/enterprise plans
        let tier = classify_plan_tier(&plan.name);
        match tier {
            PlanTier::Premium | PlanTier::Enterprise => {
                events.push(DomainEvent::NotifyPremiumSignup {
                    user_id: user.id,
                });
                events.push(DomainEvent::TrackRegistration {
                    user_id: user.id,
                    event_name: "premium_user_registered",
                });
            }
            PlanTier::Standard => {}
        }
    }

    events
}

// Pure business logic - determine if referrer should get credit
fn should_award_referral_credit(user: &User, plan: &Plan) -> bool {
    user.email_verified && !plan.is_trial
}

// Pure business logic - handle referral tracking
fn handle_referral_events(
    user: &User,
    plan: &Plan,
    has_referral: bool,
) -> Vec<DomainEvent> {
    let mut events = vec![];

    if has_referral {
        events.push(DomainEvent::TrackRegistration {
            user_id: user.id,
            event_name: "referred_user_registered",
        });

        events.push(DomainEvent::TrackReferralConversion {
            user_id: user.id,
        });

        if should_award_referral_credit(user, plan) {
            events.push(DomainEvent::AwardReferralCredit {
                user_id: user.id,
            });
        }
    }

    events
}

// Pure domain logic - main orchestration function (no I/O)
fn handle_user_registered(
    user: User,
    plan: Plan,
    event_payload: &str,
) -> Vec<DomainEvent> {
    let mut events = vec![];

    // Classify email for special handling
    let email_domain = classify_email_domain(&user.email);

    // Handle email verification
    events.extend(handle_email_verification(&user, email_domain));

    // Handle plan-specific events
    events.extend(handle_plan_events(&user, &plan));

    // Handle referral tracking
    let has_referral = has_referral_code(event_payload);
    events.extend(handle_referral_events(&user, &plan, has_referral));

    // Always track basic registration
    events.push(DomainEvent::TrackRegistration {
        user_id: user.id,
        event_name: "user_registered",
    });

    events
}

// Infrastructure layer - database, event bus, etc.
struct Database;
struct EventBus;
struct EmailService;
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
            plan_type: PlanType::Trial,
            is_trial: true,
        })
    }
}

impl EventBus {
    async fn publish(&self, _event: Event) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl EmailService {
    async fn send_welcome(
        &self,
        _user_id: i64,
        _domain: EmailDomain,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn send_verification(&self, _user_id: i64) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl Analytics {
    async fn track(&self, _event: &str, _user_id: i64) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

fn deserialize_event(event: &Event) -> Result<UserRegistered, Box<dyn Error>> {
    Ok(serde_json::from_str(&event.payload)?)
}

async fn fetch_user(user_id: i64, db: &Database) -> Result<User, Box<dyn Error>> {
    db.get_user(user_id).await
}

async fn fetch_plan(plan_id: i64, db: &Database) -> Result<Plan, Box<dyn Error>> {
    db.get_plan(plan_id).await
}

// Convert domain events to infrastructure events
// This is the only place that knows about infrastructure details
async fn publish_domain_events(
    events: Vec<DomainEvent>,
    email_service: &EmailService,
    event_bus: &EventBus,
    analytics: &Analytics,
) -> Result<(), Box<dyn Error>> {
    for event in events {
        match event {
            DomainEvent::SendWelcomeEmail { user_id, email_domain } => {
                email_service.send_welcome(user_id, email_domain).await?;

                let topic = match email_domain {
                    EmailDomain::Educational => "email.sent.educational",
                    _ => "email.sent",
                };

                event_bus
                    .publish(Event {
                        topic: topic.to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
            DomainEvent::SendVerificationEmail { user_id } => {
                email_service.send_verification(user_id).await?;
                event_bus
                    .publish(Event {
                        topic: "email.verification_needed".to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
            DomainEvent::RequestApproval { user_id, reason } => {
                event_bus
                    .publish(Event {
                        topic: "user.needs_approval".to_string(),
                        payload: format!(r#"{{"user_id":{},"reason":"{}"}}"#, user_id, reason),
                    })
                    .await?;
            }
            DomainEvent::TrackRegistration { user_id, event_name } => {
                analytics.track(event_name, user_id).await?;
            }
            DomainEvent::StartTrial { user_id, expires_at } => {
                event_bus
                    .publish(Event {
                        topic: "trial.started".to_string(),
                        payload: format!(
                            r#"{{"user_id":{},"expires_at":"{}"}}"#,
                            user_id, expires_at
                        ),
                    })
                    .await?;
            }
            DomainEvent::NotifyPremiumSignup { user_id } => {
                event_bus
                    .publish(Event {
                        topic: "user.premium_registered".to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
            DomainEvent::TrackPromoConversion { user_id } => {
                event_bus
                    .publish(Event {
                        topic: "marketing.promo_conversion".to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
            DomainEvent::TrackReferralConversion { user_id } => {
                event_bus
                    .publish(Event {
                        topic: "referral.conversion".to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
            DomainEvent::AwardReferralCredit { user_id } => {
                event_bus
                    .publish(Event {
                        topic: "referral.credit_earned".to_string(),
                        payload: format!(r#"{{"user_id":{}}}"#, user_id),
                    })
                    .await?;
            }
        }
    }
    Ok(())
}

// Infrastructure wrapper - events and I/O only
// Thin layer that coordinates infrastructure concerns
async fn on_user_registered(
    event: Event,
    db: &Database,
    email_service: &EmailService,
    event_bus: &EventBus,
    analytics: &Analytics,
) -> Result<(), Box<dyn Error>> {
    let payload = deserialize_event(&event)?;
    let user = fetch_user(payload.id, db).await?;
    let plan = fetch_plan(user.plan_id, db).await?;

    let domain_events = handle_user_registered(user, plan, &event.payload);

    publish_domain_events(domain_events, email_service, event_bus, analytics).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_email_domain() {
        assert!(matches!(
            classify_email_domain("student@university.edu"),
            EmailDomain::Educational
        ));
        assert!(matches!(
            classify_email_domain("official@agency.gov"),
            EmailDomain::Government
        ));
        assert!(matches!(
            classify_email_domain("soldier@military.mil"),
            EmailDomain::Government
        ));
        assert!(matches!(
            classify_email_domain("user@example.com"),
            EmailDomain::Regular
        ));
    }

    #[test]
    fn test_classify_plan_tier() {
        assert!(matches!(
            classify_plan_tier("Premium Plan"),
            PlanTier::Premium
        ));
        assert!(matches!(
            classify_plan_tier("Enterprise Plan"),
            PlanTier::Enterprise
        ));
        assert!(matches!(
            classify_plan_tier("Basic Plan"),
            PlanTier::Standard
        ));
    }

    #[test]
    fn test_is_promotional_plan() {
        assert!(is_promotional_plan("promo_trial"));
        assert!(is_promotional_plan("discount_plan"));
        assert!(!is_promotional_plan("regular_trial"));
    }

    #[test]
    fn test_has_referral_code() {
        assert!(has_referral_code(r#"{"id":1,"referral_code":"ABC123"}"#));
        assert!(!has_referral_code(r#"{"id":1}"#));
    }

    #[test]
    fn test_handle_email_verification_verified_regular() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };

        let events = handle_email_verification(&user, EmailDomain::Regular);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            DomainEvent::SendWelcomeEmail { user_id: 1, .. }
        ));
    }

    #[test]
    fn test_handle_email_verification_verified_educational() {
        let user = User {
            id: 1,
            email: "student@university.edu".to_string(),
            email_verified: true,
            plan_id: 1,
        };

        let events = handle_email_verification(&user, EmailDomain::Educational);

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], DomainEvent::SendWelcomeEmail { .. }));
        assert!(matches!(events[1], DomainEvent::TrackRegistration { .. }));
    }

    #[test]
    fn test_handle_email_verification_unverified() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: false,
            plan_id: 1,
        };

        let events = handle_email_verification(&user, EmailDomain::Regular);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            DomainEvent::SendVerificationEmail { user_id: 1 }
        ));
    }

    #[test]
    fn test_handle_plan_events_trial() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };
        let plan = Plan {
            id: 1,
            name: "Trial Plan".to_string(),
            plan_type: PlanType::Trial,
            is_trial: true,
        };

        let events = handle_plan_events(&user, &plan);

        assert!(events.len() >= 2); // At least trial registration and start trial
        assert!(events.iter().any(|e| matches!(e, DomainEvent::StartTrial { .. })));
    }

    #[test]
    fn test_handle_plan_events_premium_paid() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };
        let plan = Plan {
            id: 1,
            name: "Premium Plan".to_string(),
            plan_type: PlanType::Paid,
            is_trial: false,
        };

        let events = handle_plan_events(&user, &plan);

        assert!(events.iter().any(|e| matches!(e, DomainEvent::NotifyPremiumSignup { .. })));
    }

    #[test]
    fn test_should_award_referral_credit() {
        let verified_paid_user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };
        let paid_plan = Plan {
            id: 1,
            name: "Paid Plan".to_string(),
            plan_type: PlanType::Paid,
            is_trial: false,
        };
        let trial_plan = Plan {
            id: 2,
            name: "Trial Plan".to_string(),
            plan_type: PlanType::Trial,
            is_trial: true,
        };

        assert!(should_award_referral_credit(&verified_paid_user, &paid_plan));
        assert!(!should_award_referral_credit(&verified_paid_user, &trial_plan));

        let unverified_user = User {
            id: 2,
            email: "user@example.com".to_string(),
            email_verified: false,
            plan_id: 1,
        };
        assert!(!should_award_referral_credit(&unverified_user, &paid_plan));
    }

    #[test]
    fn test_handle_referral_events_with_credit() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };
        let plan = Plan {
            id: 1,
            name: "Paid Plan".to_string(),
            plan_type: PlanType::Paid,
            is_trial: false,
        };

        let events = handle_referral_events(&user, &plan, true);

        assert!(events.iter().any(|e| matches!(e, DomainEvent::AwardReferralCredit { .. })));
    }

    #[test]
    fn test_handle_user_registered_full_flow() {
        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            email_verified: true,
            plan_id: 1,
        };
        let plan = Plan {
            id: 1,
            name: "Trial Plan".to_string(),
            plan_type: PlanType::Trial,
            is_trial: true,
        };

        let events = handle_user_registered(user, plan, r#"{"id":1}"#);

        // Should have at least: welcome email, trial events, and final registration
        assert!(events.len() >= 3);
        assert!(events.iter().any(|e| matches!(
            e,
            DomainEvent::TrackRegistration { event_name: "user_registered", .. }
        )));
    }
}
