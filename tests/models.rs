mod common;

use yaima::models::{MessageResponse, Role};

#[test]
fn guest_satisfies_only_guest() {
    assert!(Role::Guest.is_at_least(Role::Guest));
    assert!(!Role::Guest.is_at_least(Role::User));
    assert!(!Role::Guest.is_at_least(Role::Admin));
}

#[test]
fn user_satisfies_guest_and_user() {
    assert!(Role::User.is_at_least(Role::Guest));
    assert!(Role::User.is_at_least(Role::User));
    assert!(!Role::User.is_at_least(Role::Admin));
}

#[test]
fn admin_satisfies_all_roles() {
    assert!(Role::Admin.is_at_least(Role::Guest));
    assert!(Role::Admin.is_at_least(Role::User));
    assert!(Role::Admin.is_at_least(Role::Admin));
}

#[test]
fn role_hierarchy_is_strictly_ordered() {
    let levels = [Role::Guest, Role::User, Role::Admin];
    for (i, lower) in levels.iter().enumerate() {
        for higher in &levels[i + 1..] {
            assert!(
                higher.is_at_least(*lower),
                "{higher:?} should beat {lower:?}"
            );
            assert!(
                !lower.is_at_least(*higher),
                "{lower:?} should not beat {higher:?}"
            );
        }
    }
}

#[test]
fn roles_serialise_to_lowercase_strings() {
    assert_eq!(serde_json::to_string(&Role::Guest).unwrap(), "\"guest\"");
    assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
    assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), "\"admin\"");
}

#[test]
fn roles_deserialise_from_lowercase_strings() {
    assert_eq!(
        serde_json::from_str::<Role>("\"guest\"").unwrap(),
        Role::Guest
    );
    assert_eq!(
        serde_json::from_str::<Role>("\"user\"").unwrap(),
        Role::User
    );
    assert_eq!(
        serde_json::from_str::<Role>("\"admin\"").unwrap(),
        Role::Admin
    );
}

#[test]
fn message_response_new_stores_message() {
    let r = MessageResponse::new("all good");
    assert_eq!(r.message, "all good");
}

#[test]
fn message_response_accepts_owned_string() {
    let msg = String::from("dynamic message");
    let r = MessageResponse::new(msg);
    assert_eq!(r.message, "dynamic message");
}
