use warp_server_auth::auth_state::AuthState;

pub(crate) fn is_feature_gated_anonymous_user_past_notebook_limit(
    auth_state: &AuthState,
    num_objects: usize,
) -> bool {
    auth_state
        .is_anonymous_user_feature_gated()
        .unwrap_or_default()
        && auth_state
            .personal_object_limits()
            .is_some_and(|limits| num_objects > limits.notebook_limit)
}
