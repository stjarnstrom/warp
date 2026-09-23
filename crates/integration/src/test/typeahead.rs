use warp::integration_testing::step::new_step_with_default_assertions;
use warp::integration_testing::terminal::util::current_shell_starter_and_version;
use warp::integration_testing::terminal::{
    assert_active_block_output_for_single_terminal_in_tab, assert_input_editor_contents,
    assert_long_running_block_executing_for_single_terminal_in_tab,
    assert_no_visible_background_blocks, wait_until_bootstrapped_single_pane_for_tab,
};
use warp::integration_testing::view_getters::single_terminal_view_for_tab;
use warp::terminal::shell::ShellType;
use warpui_core::integration::{AssertionCallback, AssertionOutcome, TestStep};

use super::{Builder, new_builder};
use crate::util::skip_if_powershell_core_2303;

pub fn test_typeahead() -> Builder {
    new_builder()
        // TODO(CORE-2732): Flakey on Powershell (Linux)
        .set_should_run_test(skip_if_powershell_core_2303)
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Execute sleep 4")
                .with_typed_characters(&["sleep 4"])
                .with_keystrokes(&["enter"])
                .add_assertion(
                    assert_long_running_block_executing_for_single_terminal_in_tab(true, 0),
                ),
        )
        .with_step(
            TestStep::new("Enter text to long running command")
                .with_input_string("foo", None)
                .add_assertion(require_long_running_block_executing(0))
                .add_assertion(assert_active_block_output_for_single_terminal_in_tab(
                    "foo", 0,
                )),
        )
        .with_step(
            new_step_with_default_assertions("Input box should have typeahead text")
                .add_assertion(assert_input_editor_contents(0, "foo"))
                .add_named_assertion(
                    "No typeahead duplicated in background block",
                    assert_no_visible_background_blocks(0, 0),
                ),
        )
}

/// Checks a typeahead command has the expected value.
///
/// PowerShell has different behavior for typeahead in that it ignores newlines.
pub fn test_input_reporting_powershell() -> Builder {
    new_builder()
        .set_should_run_test(|| {
            let (starter, _) = current_shell_starter_and_version();
            starter.shell_type() == ShellType::PowerShell
        })
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(
            TestStep::new("Execute sleep")
                .with_typed_characters(&["sleep 3"])
                .with_keystrokes(&["enter"])
                .add_assertion(
                    assert_long_running_block_executing_for_single_terminal_in_tab(true, 0),
                ),
        )
        .with_step(
            TestStep::new("Enter text to long-running command")
                .with_keystrokes(&["enter"])
                .with_input_string("true", Some(&["enter"]))
                .with_input_string("sleep 1", Some(&["enter"]))
                .add_assertion(require_long_running_block_executing(0)),
        )
        .with_step(
            new_step_with_default_assertions("Input should be reported to the terminal")
                .add_named_assertion(
                    "Typeahead is in input editor",
                    assert_input_editor_contents(0, "truesleep 1"),
                ),
        )
}

/// This tests UNIX-specific signal handling.
#[cfg(not(windows))]
pub fn test_background_output() -> Builder {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::prelude::OpenOptionsExt;

    use regex::Regex;
    use warp::integration_testing::block::assert_background_output;
    use warp::integration_testing::terminal::execute_command_for_single_terminal_in_tab;
    use warp::integration_testing::terminal::util::ExpectedExitStatus;

    let (starter, _) = current_shell_starter_and_version();
    let (spawn_command, kill_command) = match starter.shell_type() {
        ShellType::PowerShell => (
            "$process = Start-Process -FilePath './delayed_output.py' -PassThru",
            "kill -SIGUSR1 $process.Id && echo foreground",
        ),
        _ => (
            "./delayed_output.py &",
            "kill -SIGUSR1 %1 && echo foreground",
        ),
    };
    new_builder()
        .with_setup(|utils| {
            let dir = utils.test_dir();
            // Use a Python script because fish can't run functions in the background
            // https://github.com/fish-shell/fish-shell/issues/238
            let script_path = dir.join("delayed_output.py");

            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o755)
                .open(script_path)
                .expect("could not create script")
                .write_all(
                    br#"#!/usr/bin/env python3
import signal
import time

# Wait for a SIGUSR1 signal, after which we should print out
# more text.
def handler(signo, cur_frame):
  time.sleep(1)
  print("Output 2")
  print("Output 3")
signal.signal(signal.SIGUSR1, handler)

print("Output 1")
time.sleep(100)
"#,
                )
                .expect("could not write Python script");
        })
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(execute_command_for_single_terminal_in_tab(
            0,
            spawn_command.into(),
            ExpectedExitStatus::Success,
            (),
        ))
        .with_step(
            TestStep::new("First line of background output appears")
                .add_assertion(assert_background_output(0, "Output 1\n")),
        )
        .with_step(execute_command_for_single_terminal_in_tab(
            0,
            // Send the signal to the background process and produce some output.
            kill_command.into(),
            ExpectedExitStatus::Success,
            "foreground",
        ))
        .with_step(
            TestStep::new("Rest of background output appears in a new block").add_assertion(
                assert_background_output(
                    0,
                    // Use a regex because the "job completed" message format is shell-specific.
                    // Depending on timing, the "Output 2" line could be part of the
                    // block for `true`, so it's optional - we expect the next
                    // line to always be in the background block though.
                    Regex::new("^(Output 2\n)?Output 3\n").expect("Regex is valid"),
                ),
            ),
        )
}

#[cfg(windows)]
// TODO(CORE-2302): enable this test for windows
pub fn test_background_output() -> Builder {
    new_builder()
}

/// Require (as a test precondition) that a long-running block is executing.
/// Use this instead of [`assert_long_running_block_executing`] with `sleep` commands
/// to turn the race condition of the sleep ending too soon into a flake.
fn require_long_running_block_executing(tab_index: usize) -> AssertionCallback {
    Box::new(move |app, window_id| {
        let terminal_view = single_terminal_view_for_tab(app, window_id, tab_index);
        terminal_view.read(app, |view, _ctx| {
            // The running block should have received the text.
            let model = view.model.lock();
            if !model
                .block_list()
                .active_block()
                .is_active_and_long_running()
            {
                // There's an implicit race condition where the sleep call
                // can finish before we get here.
                // The most robust way of handling this is to just treat it as a flake.
                AssertionOutcome::PreconditionFailed(
                    "long-running block no longer executing".to_owned(),
                )
            } else {
                AssertionOutcome::Success
            }
        })
    })
}
