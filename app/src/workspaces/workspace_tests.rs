use super::*;

fn billing_metadata_with_purchase_policy(
    purchase_policy: Option<PurchaseAddOnCreditsPolicy>,
) -> BillingMetadata {
    let mut billing_metadata = BillingMetadata::default();
    billing_metadata.tier.purchase_add_on_credits_policy = purchase_policy;
    billing_metadata
}

#[test]
fn purchase_policy_disabled_without_policy() {
    let billing_metadata = billing_metadata_with_purchase_policy(None);

    assert!(!billing_metadata.is_purchase_add_on_credits_policy_enabled());
    assert!(!billing_metadata.is_premium_addon_credits_purchase());
    assert_eq!(billing_metadata.addon_credits_price_premium_bps(), 0);
}

#[test]
fn purchase_policy_standard_plan_has_no_premium() {
    let billing_metadata =
        billing_metadata_with_purchase_policy(Some(PurchaseAddOnCreditsPolicy {
            enabled: true,
            premium_enabled: false,
            price_premium_bps: 0,
        }));

    assert!(billing_metadata.is_purchase_add_on_credits_policy_enabled());
    assert!(!billing_metadata.is_premium_addon_credits_purchase());
    assert_eq!(billing_metadata.addon_credits_price_premium_bps(), 0);
}

#[test]
fn purchase_policy_premium_plan_enables_surcharged_purchasing() {
    let billing_metadata =
        billing_metadata_with_purchase_policy(Some(PurchaseAddOnCreditsPolicy {
            enabled: false,
            premium_enabled: true,
            price_premium_bps: 1000,
        }));

    assert!(billing_metadata.is_purchase_add_on_credits_policy_enabled());
    assert!(billing_metadata.is_premium_addon_credits_purchase());
    assert_eq!(billing_metadata.addon_credits_price_premium_bps(), 1000);
}

#[test]
fn purchase_policy_fully_disabled_plan_remains_disabled() {
    let billing_metadata =
        billing_metadata_with_purchase_policy(Some(PurchaseAddOnCreditsPolicy {
            enabled: false,
            premium_enabled: false,
            price_premium_bps: 1000,
        }));

    assert!(!billing_metadata.is_purchase_add_on_credits_policy_enabled());
    assert!(!billing_metadata.is_premium_addon_credits_purchase());
    assert_eq!(billing_metadata.addon_credits_price_premium_bps(), 0);
}

#[test]
fn purchase_policy_standard_purchasing_wins_over_premium() {
    // Standard (list price) purchasing takes precedence if the server ever
    // sends both flags; no surcharge should be displayed or applied.
    let billing_metadata =
        billing_metadata_with_purchase_policy(Some(PurchaseAddOnCreditsPolicy {
            enabled: true,
            premium_enabled: true,
            price_premium_bps: 1000,
        }));

    assert!(billing_metadata.is_purchase_add_on_credits_policy_enabled());
    assert!(!billing_metadata.is_premium_addon_credits_purchase());
    assert_eq!(billing_metadata.addon_credits_price_premium_bps(), 0);
}
