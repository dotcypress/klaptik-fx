use crate::*;

pub const FLASH_MAX_ADDRESS: usize = 0x1ffff;
pub const FLASH_ADDR_BYTES: usize = 3;
pub const FLASH_PAGE_SIZE: usize = 256;

pub const KVS_MAGIC: u32 = 0x4242;
pub const KVS_NONCE: u16 = 0;
pub const KVS_BUCKETS: usize = 512;
pub const KVS_SLOTS: usize = 16;
pub const KVS_MAX_HOPS: usize = 32;

pub type FlashStoreAdapter =
    PagedAdapter<SpiStoreAdapter<SharedBus<SpiDev>, EepromCS, FLASH_ADDR_BYTES>, FLASH_PAGE_SIZE>;
pub type FlashStore = KVStore<FlashStoreAdapter, KVS_BUCKETS, KVS_SLOTS>;
