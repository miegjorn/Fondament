use fondament_core::address::CompositionAddress;

#[test]
fn parses_role_address() {
    let a: CompositionAddress = "fondament/app-architect".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "fondament/app-architect");
            assert!(modifiers.is_empty());
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role"),
    }
}

#[test]
fn parses_composed_address_with_facet() {
    let a: CompositionAddress = "acme-auth/auth+adversarial".parse().unwrap();
    match &a {
        CompositionAddress::Composed { project, facet, modifiers, stance } => {
            assert_eq!(project, "acme-auth");
            assert_eq!(facet.as_deref(), Some("auth"));
            assert!(modifiers.is_empty());
            assert_eq!(stance, "adversarial");
        }
        _ => panic!("expected Composed"),
    }
}

#[test]
fn display_roundtrips() {
    for s in ["fondament/app-architect", "proj/facet+builder"] {
        let a: CompositionAddress = s.parse().unwrap();
        assert_eq!(a.to_string(), s);
    }
}

#[test]
fn parses_role_with_deconstructive_modifier() {
    let a: CompositionAddress = "fondament/roles/security-sre+deconstructive".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "fondament/roles/security-sre");
            assert_eq!(modifiers, &["deconstructive"]);
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role"),
    }
}

#[test]
fn parses_composed_with_modifier_and_stance() {
    let a: CompositionAddress = "acme-auth/auth+deconstructive+adversarial".parse().unwrap();
    match &a {
        CompositionAddress::Composed { project, facet, modifiers, stance } => {
            assert_eq!(project, "acme-auth");
            assert_eq!(facet.as_deref(), Some("auth"));
            assert_eq!(modifiers, &["deconstructive"]);
            assert_eq!(stance, "adversarial");
        }
        _ => panic!("expected Composed"),
    }
}

#[test]
fn parses_modifier_only_non_fondament_as_role() {
    // No stance means no Composed — falls through to Role
    let a: CompositionAddress = "acme-auth/auth+deconstructive".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, modifiers, stance_override } => {
            assert_eq!(role, "acme-auth/auth");
            assert_eq!(modifiers, &["deconstructive"]);
            assert!(stance_override.is_none());
        }
        _ => panic!("expected Role (modifier-only, no stance)"),
    }
}

#[test]
fn display_roundtrips_with_modifier() {
    for s in [
        "fondament/roles/security-sre+deconstructive",
        "acme-auth/auth+deconstructive+adversarial",
    ] {
        let a: CompositionAddress = s.parse().unwrap();
        assert_eq!(a.to_string(), s, "display must roundtrip for {}", s);
    }
}
