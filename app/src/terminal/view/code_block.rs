use warpui::elements::{
    Container, CornerRadius, CrossAxisAlignment, Flex, MainAxisAlignment, MainAxisSize,
    MouseStateHandle, ParentElement, Radius, Text,
};
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Element, EventContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::ui_components::blended_colors;
use crate::ui_components::buttons::icon_button;
use crate::ui_components::icons::Icon;

const CODE_BLOCK_CORNER_RADIUS: f32 = 8.0;
const CODE_BLOCK_HORIZONTAL_PADDING: f32 = 16.;
const CODE_BLOCK_VERTICAL_PADDING: f32 = 10.;

#[derive(Default, Clone)]
pub struct CodeSnippetButtonHandles {
    pub copy_button: MouseStateHandle,
    pub execute_button: MouseStateHandle,
}

pub type HandleCode = Box<dyn FnMut(String, &mut EventContext)>;

fn render_button(
    appearance: &Appearance,
    icon: Icon,
    tooltip_text: &'static str,
    mouse_handle: MouseStateHandle,
    code: String,
    mut on_click: HandleCode,
) -> Container {
    let ui_builder = appearance.ui_builder().clone();
    Container::new(
        icon_button(appearance, icon, false, mouse_handle)
            .with_tooltip(move || {
                ui_builder
                    .tool_tip(tooltip_text.to_owned())
                    .build()
                    .finish()
            })
            .build()
            .on_click(move |ctx, _, _| on_click(code.clone(), ctx))
            .finish(),
    )
}

/// Renders `code` in a monospace block with copy and (optionally) run-in-terminal buttons.
pub fn render_code_block_plain(
    code: &str,
    on_copy: HandleCode,
    on_execute: Option<HandleCode>,
    handles: CodeSnippetButtonHandles,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();

    let code_element = Text::new(
        code.to_owned(),
        appearance.monospace_font_family(),
        appearance.monospace_font_size(),
    )
    .with_color(blended_colors::text_main(theme, theme.surface_1()))
    .with_selectable(true)
    .finish();

    let mut action_row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(
            render_button(
                appearance,
                Icon::Copy,
                "Copy",
                handles.copy_button,
                code.to_owned(),
                on_copy,
            )
            .finish(),
        );
    if let Some(on_execute) = on_execute {
        action_row.add_child(
            render_button(
                appearance,
                Icon::TerminalInput,
                "Run in terminal",
                handles.execute_button,
                code.to_owned(),
                on_execute,
            )
            .with_margin_left(8.)
            .finish(),
        );
    }

    Flex::column()
        .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
        .with_child(
            Container::new(code_element)
                .with_background(theme.surface_2())
                .with_vertical_padding(CODE_BLOCK_VERTICAL_PADDING)
                .with_horizontal_padding(CODE_BLOCK_HORIZONTAL_PADDING)
                .with_corner_radius(CornerRadius::with_top(Radius::Pixels(
                    CODE_BLOCK_CORNER_RADIUS,
                )))
                .finish(),
        )
        .with_child(
            Container::new(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::End)
                    .with_child(action_row.finish())
                    .finish(),
            )
            .with_background(theme.surface_2())
            .with_padding_bottom(12.)
            .with_horizontal_padding(CODE_BLOCK_HORIZONTAL_PADDING)
            .with_corner_radius(CornerRadius::with_bottom(Radius::Pixels(
                CODE_BLOCK_CORNER_RADIUS,
            )))
            .finish(),
        )
        .finish()
}
