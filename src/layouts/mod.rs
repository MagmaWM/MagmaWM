
pub trait Layout {
    type WindowContainer;

    fn update_layout(window_container: &mut WindowContainer);
}
