use pathfinder_color::ColorU;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::ContrastingColor;
use warp_core::ui::color::contrast::MinimumAllowedContrast;
use warp_core::ui::theme::Fill;

pub fn on_surface(appearance: &Appearance, color: Fill) -> ColorU {
    color
        .on_background(
            appearance.theme().surface_1(),
            MinimumAllowedContrast::NonText,
        )
        .into()
}
