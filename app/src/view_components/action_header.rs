//! A header row for blocklist rich content that shows a status icon, a title and optional action
//! buttons or an expansion toggle.
use std::borrow::Cow;
use std::rc::Rc;

use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::AnsiColorIdentifier;
use warpui::elements::{
    ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Expanded, Flex, Hoverable,
    MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius, Shrinkable,
    SizeConstraintCondition, SizeConstraintSwitch, Text,
};
use warpui::platform::Cursor;
use warpui::{AppContext, Element, EventContext, SingletonEntity};

use crate::ui_components::blended_colors;
use crate::ui_components::icons::Icon;
use crate::view_components::compactible_action_button::{
    RenderCompactibleActionButton, render_compact_and_regular_button_rows, render_expansion_icon,
};

pub const HEADER_HORIZONTAL_PADDING: f32 = 16.;
const HEADER_VERTICAL_PADDING: f32 = 10.;
const ICON_MARGIN: f32 = 8.;

/// Returns the size for header icons, scaled to the user's current font size.
pub fn icon_size(app: &AppContext) -> f32 {
    let appearance = Appearance::as_ref(app);
    app.font_cache().line_height(
        appearance.monospace_font_size(),
        appearance.line_height_ratio(),
    )
}

pub fn green_check_icon(appearance: &Appearance) -> warpui::elements::Icon {
    warpui::elements::Icon::new(
        Icon::Check.into(),
        AnsiColorIdentifier::Green.to_ansi_color(&appearance.theme().terminal_colors().normal),
    )
}

pub fn red_x_icon(appearance: &Appearance) -> warpui::elements::Icon {
    warpui::elements::Icon::new(
        Icon::X.into(),
        AnsiColorIdentifier::Red.to_ansi_color(&appearance.theme().terminal_colors().normal),
    )
}

pub fn cancelled_icon(appearance: &Appearance) -> warpui::elements::Icon {
    warpui::elements::Icon::new(
        Icon::Cancelled.into(),
        blended_colors::neutral_6(appearance.theme()),
    )
}

pub fn yellow_stop_icon(appearance: &Appearance) -> warpui::elements::Icon {
    warpui::elements::Icon::new(
        Icon::StopFilled.into(),
        AnsiColorIdentifier::Yellow.to_ansi_color(&appearance.theme().terminal_colors().normal),
    )
}

pub fn yellow_running_icon(appearance: &Appearance) -> warpui::elements::Icon {
    warpui::elements::Icon::new(
        Icon::Circle.into(),
        AnsiColorIdentifier::Yellow.to_ansi_color(&appearance.theme().terminal_colors().normal),
    )
}

pub type OnToggleExpandedCallback = Rc<dyn Fn(&mut EventContext) + 'static>;

/// Configuration for a header that can be manually expanded and collapsed.
#[derive(Clone)]
pub struct ExpandedConfig {
    pub is_expanded: bool,
    pub on_toggle_expanded: Option<OnToggleExpandedCallback>,
    pub toggle_mouse_state: MouseStateHandle,
}

impl ExpandedConfig {
    pub fn new(is_expanded: bool, toggle_mouse_state: MouseStateHandle) -> Self {
        Self {
            is_expanded,
            on_toggle_expanded: None,
            toggle_mouse_state,
        }
    }

    pub fn with_toggle_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut EventContext) + 'static,
    {
        self.on_toggle_expanded = Some(Rc::new(callback));
        self
    }
}

#[derive(Clone)]
pub enum InteractionMode {
    /// Renders action buttons, switching to compact buttons below the given width.
    ActionButtons {
        action_buttons: Vec<Rc<dyn RenderCompactibleActionButton>>,
        size_switch_threshold: f32,
    },
    /// Renders an expansion chevron with a caller-specified click handler.
    ManuallyExpandable(ExpandedConfig),
}

#[derive(Clone)]
pub struct HeaderConfig {
    pub title: Cow<'static, str>,
    pub icon: Option<warpui::elements::Icon>,
    pub interaction_mode: Option<InteractionMode>,
    pub is_text_selectable: bool,
}

impl HeaderConfig {
    pub fn new(title: impl Into<Cow<'static, str>>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            interaction_mode: None,
            is_text_selectable: false,
        }
    }

    pub fn with_icon(mut self, icon: warpui::elements::Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_interaction_mode(mut self, interaction_mode: InteractionMode) -> Self {
        self.interaction_mode = Some(interaction_mode);
        self
    }

    pub fn with_selectable_text(mut self) -> Self {
        self.is_text_selectable = true;
        self
    }

    fn render_header(
        self,
        app: &AppContext,
        interaction_mode_content: Option<Box<dyn Element>>,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let header_background = appearance.theme().surface_2();

        let mut header_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        let mut left_content_container = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Start)
            .with_cross_axis_alignment(CrossAxisAlignment::Center);

        if let Some(icon) = self.icon {
            left_content_container.add_child(
                Container::new(
                    ConstrainedBox::new(icon.finish())
                        .with_width(icon_size(app))
                        .with_height(icon_size(app))
                        .finish(),
                )
                .with_margin_right(ICON_MARGIN)
                .finish(),
            )
        }

        let title_element = Text::new_inline(
            self.title.clone(),
            appearance.ui_font_family(),
            appearance.monospace_font_size(),
        )
        .with_selectable(self.is_text_selectable)
        .with_color(blended_colors::text_main(
            appearance.theme(),
            header_background,
        ))
        .finish();

        left_content_container.add_child(
            Expanded::new(
                1.,
                Container::new(title_element).with_margin_right(8.).finish(),
            )
            .finish(),
        );

        header_row.add_child(Shrinkable::new(1., left_content_container.finish()).finish());

        if let Some(interaction_mode_content) = interaction_mode_content {
            header_row.add_child(interaction_mode_content);
        }

        let looks_expanded_downwards =
            self.interaction_mode
                .as_ref()
                .is_some_and(|mode| match mode {
                    InteractionMode::ActionButtons { .. } => true,
                    InteractionMode::ManuallyExpandable(expansion_config) => {
                        expansion_config.is_expanded
                    }
                });
        let container = Container::new(header_row.finish())
            .with_horizontal_padding(HEADER_HORIZONTAL_PADDING)
            .with_vertical_padding(HEADER_VERTICAL_PADDING)
            .with_background(header_background)
            .with_corner_radius(if looks_expanded_downwards {
                CornerRadius::with_top(Radius::Pixels(8.))
            } else {
                CornerRadius::with_all(Radius::Pixels(8.))
            })
            .finish();

        if let Some(InteractionMode::ManuallyExpandable(expansion_config)) = &self.interaction_mode
            && let Some(callback) = &expansion_config.on_toggle_expanded
        {
            let callback = Rc::clone(callback);
            return Hoverable::new(expansion_config.toggle_mouse_state.clone(), |_| container)
                .on_click(move |ctx, _, _| callback(ctx))
                .with_cursor(Cursor::PointingHand)
                .finish();
        }

        container
    }

    pub fn render(self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        match self.interaction_mode.clone() {
            Some(InteractionMode::ActionButtons {
                action_buttons,
                size_switch_threshold,
            }) => {
                let button_refs: Vec<&dyn RenderCompactibleActionButton> =
                    action_buttons.iter().map(|b| b.as_ref()).collect();

                let (regular_row, compact_row) =
                    render_compact_and_regular_button_rows(button_refs, None, appearance, app);

                let regular_header = self.clone().render_header(app, Some(regular_row));
                let compact_header = self.render_header(app, Some(compact_row));

                SizeConstraintSwitch::new(
                    regular_header,
                    vec![(
                        SizeConstraintCondition::WidthLessThan(
                            size_switch_threshold * appearance.monospace_ui_scalar(),
                        ),
                        compact_header,
                    )],
                )
                .finish()
            }
            Some(InteractionMode::ManuallyExpandable(expansion_config)) => {
                let expanded_icon = ConstrainedBox::new(render_expansion_icon(
                    expansion_config.is_expanded,
                    false,
                    appearance,
                    app,
                ))
                .with_height(icon_size(app))
                .with_width(icon_size(app))
                .finish();

                self.render_header(app, Some(expanded_icon))
            }
            None => self.render_header(app, None),
        }
    }
}
