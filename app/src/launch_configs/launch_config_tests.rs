use std::path::PathBuf;

use super::{CommandTemplate, LaunchConfig, PaneMode, PaneTemplateType, TabTemplate};
use crate::app_state::SplitDirection;

#[test]
fn test_tab_level_commands_are_applied_to_leaf_layout() {
    let config: LaunchConfig = serde_yaml::from_str(
        r#"
name: Legacy Commands
windows:
  - tabs:
      - layout:
          cwd: /tmp
        commands:
          - exec: echo hello
"#,
    )
    .expect("launch config should parse");

    let layout = config.windows[0].tabs[0].layout_with_tab_commands();

    assert_eq!(
        layout,
        PaneTemplateType::PaneTemplate {
            cwd: PathBuf::from("/tmp"),
            commands: vec![CommandTemplate {
                exec: "echo hello".to_string()
            }],
            is_focused: None,
            pane_mode: PaneMode::Terminal,
            shell: None,
        }
    );
}

#[test]
fn test_tab_level_commands_are_applied_to_focused_pane_in_branch_layout() {
    let config: LaunchConfig = serde_yaml::from_str(
        r#"
name: Legacy Commands
windows:
  - tabs:
      - layout:
          split_direction: horizontal
          panes:
            - cwd: /tmp/left
              is_focused: false
            - cwd: /tmp/right
              is_focused: true
        commands:
          - exec: echo focused
"#,
    )
    .expect("launch config should parse");

    let layout = config.windows[0].tabs[0].layout_with_tab_commands();

    assert_eq!(
        layout,
        PaneTemplateType::PaneBranchTemplate {
            split_direction: SplitDirection::Horizontal.into(),
            panes: vec![
                PaneTemplateType::PaneTemplate {
                    cwd: PathBuf::from("/tmp/left"),
                    commands: vec![],
                    is_focused: Some(false),
                    pane_mode: PaneMode::Terminal,
                    shell: None,
                },
                PaneTemplateType::PaneTemplate {
                    cwd: PathBuf::from("/tmp/right"),
                    commands: vec![CommandTemplate {
                        exec: "echo focused".to_string()
                    }],
                    is_focused: Some(true),
                    pane_mode: PaneMode::Terminal,
                    shell: None,
                },
            ],
        }
    );
}

#[test]
fn test_tab_level_commands_are_applied_to_first_pane_without_focused_pane() {
    let config: LaunchConfig = serde_yaml::from_str(
        r#"
name: Legacy Commands
windows:
  - tabs:
      - layout:
          split_direction: horizontal
          panes:
            - cwd: /tmp/left
            - cwd: /tmp/right
        commands:
          - exec: echo first
"#,
    )
    .expect("launch config should parse");

    let layout = config.windows[0].tabs[0].layout_with_tab_commands();

    assert_eq!(
        layout,
        PaneTemplateType::PaneBranchTemplate {
            split_direction: SplitDirection::Horizontal.into(),
            panes: vec![
                PaneTemplateType::PaneTemplate {
                    cwd: PathBuf::from("/tmp/left"),
                    commands: vec![CommandTemplate {
                        exec: "echo first".to_string()
                    }],
                    is_focused: None,
                    pane_mode: PaneMode::Terminal,
                    shell: None,
                },
                PaneTemplateType::PaneTemplate {
                    cwd: PathBuf::from("/tmp/right"),
                    commands: vec![],
                    is_focused: None,
                    pane_mode: PaneMode::Terminal,
                    shell: None,
                },
            ],
        }
    );
}

// ---------------------------------------------------------------------------
// Tab groups (#13898)
// ---------------------------------------------------------------------------

fn tab_in_group(group: Option<usize>) -> TabTemplate {
    TabTemplate {
        title: None,
        layout: PaneTemplateType::PaneTemplate {
            cwd: PathBuf::from("/tmp"),
            commands: vec![],
            is_focused: None,
            pane_mode: PaneMode::Terminal,
            shell: None,
        },
        commands: vec![],
        color: None,
        group,
    }
}

#[test]
fn test_resolve_group_memberships_keeps_contiguous_runs_intact() {
    let tabs = vec![
        tab_in_group(Some(0)),
        tab_in_group(Some(0)),
        tab_in_group(None),
        tab_in_group(Some(1)),
    ];

    assert_eq!(
        super::resolve_group_memberships(&tabs, 2),
        vec![Some(0), Some(0), None, Some(1)]
    );
}

#[test]
fn test_resolve_group_memberships_ungroups_a_split_run() {
    // The tab bar renders each contiguous run as its own container, so
    // honoring the second run would draw two containers with one group id.
    let tabs = vec![
        tab_in_group(Some(0)),
        tab_in_group(None),
        tab_in_group(Some(0)),
    ];

    assert_eq!(
        super::resolve_group_memberships(&tabs, 1),
        vec![Some(0), None, None],
        "the group's second run must not reopen it"
    );
}

#[test]
fn test_resolve_group_memberships_ungroups_a_run_split_by_another_group() {
    let tabs = vec![
        tab_in_group(Some(0)),
        tab_in_group(Some(1)),
        tab_in_group(Some(0)),
    ];

    assert_eq!(
        super::resolve_group_memberships(&tabs, 2),
        vec![Some(0), Some(1), None]
    );
}

#[test]
fn test_resolve_group_memberships_drops_out_of_range_indices() {
    // Hand-edited YAML pointing past the end of `tab_groups`.
    let tabs = vec![tab_in_group(Some(7)), tab_in_group(Some(0))];

    assert_eq!(
        super::resolve_group_memberships(&tabs, 1),
        vec![None, Some(0)]
    );
}
