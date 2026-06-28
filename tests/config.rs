use socle::config::{OAuthConfig, OAuthProvider, OAuthProviderConfig};

#[test]
fn oauth_provider_from_slug_returns_some_for_known() {
    assert_eq!(
        OAuthProvider::from_slug("google"),
        Some(OAuthProvider::Google)
    );
    assert_eq!(
        OAuthProvider::from_slug("github"),
        Some(OAuthProvider::GitHub)
    );
}

#[test]
fn oauth_provider_from_slug_returns_none_for_unknown() {
    assert!(OAuthProvider::from_slug("unknown").is_none());
    assert!(OAuthProvider::from_slug("gitlab").is_none());
}

#[test]
fn oauth_provider_slug_returns_correct_strings() {
    assert_eq!(OAuthProvider::Google.slug(), "google");
    assert_eq!(OAuthProvider::GitHub.slug(), "github");
}

#[test]
fn oauth_provider_display_returns_slug() {
    assert_eq!(format!("{}", OAuthProvider::Google), "google");
    assert_eq!(format!("{}", OAuthProvider::GitHub), "github");
}

#[test]
fn oauth_config_provider_returns_none_when_not_configured() {
    let cfg = OAuthConfig::default();
    assert!(cfg.provider(OAuthProvider::Google).is_none());
    assert!(cfg.provider(OAuthProvider::GitHub).is_none());
}

#[test]
fn oauth_config_provider_returns_some_when_configured() {
    let cfg = OAuthConfig {
        google: Some(OAuthProviderConfig {
            client_id: "id".into(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost/callback".into(),
        }),
        github: None,
    };
    assert!(cfg.provider(OAuthProvider::Google).is_some());
    assert!(cfg.provider(OAuthProvider::GitHub).is_none());
}
