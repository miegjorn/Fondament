use fondament_core::address::CompositionAddress;

#[test]
fn parses_role_address() {
    let a: CompositionAddress = "fondament/app-architect".parse().unwrap();
    match &a {
        CompositionAddress::Role { role, .. } => assert_eq!(role, "fondament/app-architect"),
        _ => panic!(),
    }
}

#[test]
fn parses_composed_address_with_facet() {
    let a: CompositionAddress = "acme-auth/auth+adversarial".parse().unwrap();
    match &a {
        CompositionAddress::Composed { project, facet, stance } => {
            assert_eq!(project, "acme-auth");
            assert_eq!(facet.as_deref(), Some("auth"));
            assert_eq!(stance, "adversarial");
        }
        _ => panic!(),
    }
}

#[test]
fn display_roundtrips() {
    for s in ["fondament/app-architect", "proj/facet+builder"] {
        let a: CompositionAddress = s.parse().unwrap();
        assert_eq!(a.to_string(), s);
    }
}
