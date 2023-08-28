use klaptik::*;

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
            include_bytes!("background.bin"),
        ),
        Glyphs::Sequential(2),
    ),
    (
        FlashSprite::new(
            Asset::FontSmall as _,
            11,
            Size::new(10, 16),
            include_bytes!("font_small.bin"),
        ),
        Glyphs::Alphabet(b" 0123456789"),
    ),
    (
        FlashSprite::new(
            Asset::FontLarge as _,
            11,
            Size::new(36, 32),
            include_bytes!("font_large.bin"),
        ),
        Glyphs::Alphabet(b" 0123456789"),
    ),
];
