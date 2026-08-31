use token_tracing_widget_lib::providers::registry::provider_registry;
use token_tracing_widget_lib::types::provider::Provider;

#[test]
fn registry_exposes_canonical_provider_order_and_matching_adapters() {
    let registry = provider_registry();

    assert_eq!(
        registry.providers().collect::<Vec<_>>(),
        Provider::all().to_vec()
    );
    for provider in Provider::all() {
        let registration = registry
            .registration(*provider)
            .expect("every canonical provider should be registered");
        assert_eq!(registration.provider(), *provider);
        assert_eq!(registration.adapter().provider(), *provider);
    }
}
