use warp_multi_agent_api as api;
use warp_multi_agent_api::AgentType;

use super::BaseUserQuery;
use crate::ai::agent::UserQueryMode;

fn base(query: api::request::input::UserQuery) -> BaseUserQuery {
    BaseUserQuery::from_proto(query)
}

fn plan_mode() -> api::UserQueryMode {
    api::UserQueryMode {
        r#type: Some(api::user_query_mode::Type::Plan(())),
    }
}

fn normal_mode() -> api::UserQueryMode {
    api::UserQueryMode { r#type: None }
}

#[test]
fn debug_output_reports_shape_without_content() {
    let query = api::request::input::UserQuery {
        query: "the secret launch codes are 1234".to_string(),
        origin: Some(api::UserQueryOrigin::default()),
        ..Default::default()
    };

    let debug = format!("{:?}", BaseUserQuery::from_proto(query));

    assert!(!debug.contains("secret"), "{debug}");
    assert!(debug.contains("query_len: 32"), "{debug}");
    assert!(debug.contains("has_origin: true"), "{debug}");
    assert!(debug.contains("has_author: false"), "{debug}");
}

#[test]
fn accessors_report_unset_fields_as_none() {
    let unset = base(Default::default());
    assert_eq!(unset.query(), None);
    assert_eq!(unset.user_query_mode(), None);
    assert_eq!(unset.intended_agent(), None);

    let set = base(api::request::input::UserQuery {
        query: "look at the failing test".to_string(),
        mode: Some(plan_mode()),
        intended_agent: AgentType::Cli.into(),
        ..Default::default()
    });
    assert_eq!(set.query(), Some("look at the failing test"));
    assert_eq!(set.user_query_mode(), Some(UserQueryMode::Plan));
    assert_eq!(set.intended_agent(), Some(AgentType::Cli));

    // An explicitly Normal mode is a classification, not an unset field.
    let normal = base(api::request::input::UserQuery {
        mode: Some(normal_mode()),
        ..Default::default()
    });
    assert_eq!(normal.user_query_mode(), Some(UserQueryMode::Normal));
}

#[test]
fn seed_uses_the_server_text_and_normalizes_a_prefix_the_server_did_not_classify() {
    let seeded = base(api::request::input::UserQuery {
        query: "/plan from the server".to_string(),
        intended_agent: AgentType::Cli.into(),
        ..Default::default()
    })
    .seed_input_fields(
        "from the prompt".to_string(),
        UserQueryMode::Normal,
        Some(AgentType::Primary),
    );

    assert_eq!(
        seeded,
        (
            "from the server".to_string(),
            UserQueryMode::Plan,
            Some(AgentType::Cli)
        )
    );
}

#[test]
fn seed_falls_back_to_the_client_for_fields_the_server_left_unset() {
    let seeded = base(Default::default()).seed_input_fields(
        "from the prompt".to_string(),
        UserQueryMode::Plan,
        Some(AgentType::Primary),
    );

    assert_eq!(
        seeded,
        (
            "from the prompt".to_string(),
            UserQueryMode::Plan,
            Some(AgentType::Primary)
        )
    );
}

#[test]
fn seed_prefers_the_server_mode_when_it_sent_no_text() {
    let seeded = base(api::request::input::UserQuery {
        mode: Some(plan_mode()),
        ..Default::default()
    })
    .seed_input_fields("from the prompt".to_string(), UserQueryMode::Normal, None);

    assert_eq!(
        seeded,
        ("from the prompt".to_string(), UserQueryMode::Plan, None)
    );
}

#[test]
fn seed_strips_a_prefix_that_agrees_with_the_server_mode() {
    let seeded = base(api::request::input::UserQuery {
        query: "/plan foo".to_string(),
        mode: Some(plan_mode()),
        ..Default::default()
    })
    .seed_input_fields(String::new(), UserQueryMode::Normal, None);

    assert_eq!(seeded, ("foo".to_string(), UserQueryMode::Plan, None));
}

#[test]
fn seed_keeps_the_text_verbatim_when_the_server_classified_the_mode_differently() {
    let seeded = base(api::request::input::UserQuery {
        query: "/plan foo".to_string(),
        mode: Some(normal_mode()),
        ..Default::default()
    })
    .seed_input_fields(String::new(), UserQueryMode::Plan, None);

    assert_eq!(
        seeded,
        ("/plan foo".to_string(), UserQueryMode::Normal, None)
    );
}

fn external_author() -> api::QueryAuthor {
    api::QueryAuthor {
        principal: Some(api::query_author::Principal::User(api::WarpUser {
            uid: "external-author".into(),
            email: "author@example.com".into(),
            team_uid: "author-team".into(),
        })),
        resolution: api::IdentityResolution::ExternalAccountBinding.into(),
    }
}

fn external_source() -> api::ExternalMessage {
    api::ExternalMessage {
        body: "new message".into(),
        platform: Some(api::external_message::Platform::Slack(
            api::external_message::Slack {
                channel_id: "channel".into(),
                thread_ts: "thread".into(),
                thread_history: vec![api::external_message::slack::ThreadMessage {
                    text: "earlier context".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        )),
        ..Default::default()
    }
}

fn external_origin() -> api::UserQueryOrigin {
    api::UserQueryOrigin {
        variant: Some(api::user_query_origin::Variant::ExternalPlatform(
            api::user_query_origin::ExternalPlatform {},
        )),
    }
}

#[test]
fn from_message_lifts_only_the_attribution_and_skips_unattributed_messages() {
    assert!(BaseUserQuery::from_message(&api::message::UserQuery::default()).is_none());

    let message = api::message::UserQuery {
        query: "formatted follow-up".into(),
        mode: Some(api::UserQueryMode { r#type: None }),
        origin: Some(external_origin()),
        author: Some(external_author()),
        source_message: Some(external_source()),
        ..Default::default()
    };

    let lifted = BaseUserQuery::from_message(&message)
        .expect("an attributed message is lifted")
        .to_proto();

    assert_eq!(lifted.origin, Some(external_origin()));
    assert_eq!(lifted.author, Some(external_author()));
    assert_eq!(lifted.source_message, Some(external_source()));
    assert!(
        lifted.query.is_empty(),
        "text and mode are the live input's, not the message's"
    );
    assert!(lifted.mode.is_none());
}

#[test]
fn from_message_keeps_an_unresolved_sender() {
    let message = api::message::UserQuery {
        author: Some(api::QueryAuthor {
            principal: None,
            resolution: api::IdentityResolution::Unresolved.into(),
        }),
        ..Default::default()
    };

    let lifted = BaseUserQuery::from_message(&message).unwrap().to_proto();

    assert_eq!(
        lifted.author.unwrap().resolution,
        api::IdentityResolution::Unresolved as i32
    );
}
