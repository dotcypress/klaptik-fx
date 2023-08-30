use super::{assets::*, *};
use core::fmt::Write;
use klaptik::*;

widget_group! {
    UI<&AppState>,
    {
        bg: GlyphIcon, Asset::Background, 1, Point::zero();
        label0: Label<2>, Asset::FontLarge, "00", Point::new(2, 24), Size::new(36, 32);
        label1: Label<2>, Asset::FontSmall, "00", Point::new(18, 0), Size::new(10, 16);
        label2: Label<2>, Asset::FontSmall, "00", Point::new(76, 0), Size::new(10, 16);
    },
    |widget: &mut UI, state: &AppState| {
        write!(widget.label0, "{:0>2}", state.frame % 100).ok();
        write!(widget.label1, "{:0>2}", state.frame % 77).ok();
        write!(widget.label2, "{:0>2}", state.frame % 33).ok();
    }
}
