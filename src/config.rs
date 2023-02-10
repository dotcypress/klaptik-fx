pub const I2C_ADDRESS: u16 = 0x2a;
pub const FX_ADDRESS: u16 = I2C_ADDRESS;
pub const RENDER_ADDRESS: u16 = I2C_ADDRESS | 1;

pub const SPRITE_CACHE_SIZE: usize = 128;

pub const FLASH_MAX_ADDRESS: usize = 0x1ffff;
pub const FLASH_ADDR_BYTES: usize = 3;
pub const FLASH_PAGE_SIZE: usize = 256;

pub const KVS_MAGIC: u32 = 0x2a2b;
pub const KVS_NONCE: u16 = 45_033;
pub const KVS_BUCKETS: usize = 512;
pub const KVS_SLOTS: usize = 16;
pub const KVS_MAX_HOPS: usize = 32;
