use crate::*;
use core::fmt::Write;

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

pub enum Asset {
    Background = 0,
    FontSmall = 1,
    FontLarge = 2,
}

impl From<Asset> for SpriteId {
    fn from(asset: Asset) -> Self {
        asset as _
    }
}

pub const SPRITES: [(FlashSprite, Glyphs); 3] = [
    (
        FlashSprite::new(
            Asset::Background as _,
            2,
            Size::new(128, 64),
            include_bytes!("assets/background.bin"),
        ),
        Glyphs::Sequential(2),
    ),
    (
        FlashSprite::new(
            Asset::FontSmall as _,
            11,
            Size::new(10, 16),
            include_bytes!("assets/font_small.bin"),
        ),
        Glyphs::Alphabet(b" 0123456789"),
    ),
    (
        FlashSprite::new(
            Asset::FontLarge as _,
            11,
            Size::new(36, 32),
            include_bytes!("assets/font_large.bin"),
        ),
        Glyphs::Alphabet(b" 0123456789"),
    ),
];
