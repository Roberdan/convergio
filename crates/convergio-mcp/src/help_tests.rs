use crate::help::response;
use convergio_api::{Action, HelpRequest, HelpTopic, HelpVerbosity};
use serde_json::Value;

fn action_request(action: Action) -> HelpRequest {
    HelpRequest {
        topic: HelpTopic::Action,
        action: Some(action),
        verbosity: HelpVerbosity::Short,
    }
}

#[test]
fn action_help_covers_every_catalog_action() {
    for action in Action::ALL {
        let help = response(&action_request(*action));
        assert!(
            help.get("error").is_none(),
            "{} lacks action help: {help}",
            action.as_str()
        );
        assert!(
            help.get("params").is_some(),
            "{} action help lacks params: {help}",
            action.as_str()
        );
    }
}

#[test]
fn actions_topic_matches_api_catalog() {
    let help = response(&HelpRequest {
        topic: HelpTopic::Actions,
        action: None,
        verbosity: HelpVerbosity::Short,
    });
    let actions = help.get("actions").and_then(Value::as_array).unwrap();
    assert_eq!(actions.len(), Action::ALL.len());
    for action in Action::ALL {
        assert!(actions
            .iter()
            .any(|value| { value.get("name").and_then(Value::as_str) == Some(action.as_str()) }));
    }
}
