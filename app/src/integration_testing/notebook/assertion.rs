use warpui::async_assert;
use warpui::integration::AssertionCallback;

use crate::integration_testing::view_getters::terminal_view;

pub fn assert_open_in_warp_banner_open(tab_index: usize, pane_index: usize) -> AssertionCallback {
    Box::new(move |app, window_id| {
        let terminal = terminal_view(app, window_id, tab_index, pane_index);
        terminal.read(app, |view, _ctx| {
            async_assert!(
                view.is_open_in_warp_banner_open(),
                "Expected the 'Open in Warp' banner to be open"
            )
        })
    })
}
