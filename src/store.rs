use crate::*;
use kvs::adapters::paged::PagedAdapter;
use kvs::adapters::spi::*;
use kvs::*;

pub const FLASH_MAX_ADDRESS: usize = 0xffff;
pub const FLASH_ADDR_BYTES: usize = 2;
pub const FLASH_PAGE_SIZE: usize = 128;

pub const KVS_MAGIC: u32 = 0x4242;
pub const KVS_NONCE: u16 = 0;
pub const KVS_BUCKETS: usize = 128;
pub const KVS_SLOTS: usize = 16;
pub const KVS_MAX_HOPS: usize = 32;

pub type FlashStoreAdapter =
    PagedAdapter<SpiStoreAdapter<SharedBus<SpiDev>, EepromCS, FLASH_ADDR_BYTES>, FLASH_PAGE_SIZE>;
pub type FlashStore = KVStore<FlashStoreAdapter, KVS_BUCKETS, KVS_SLOTS>;

pub struct Store {
    pub store: FlashStore,
    pub wp: EepromWP,
}

impl Store {
    pub fn new(spi: SharedBus<SpiDev>, cs: EepromCS, wp: EepromWP) -> Self {
        let store_adapter = FlashStoreAdapter::new(SpiStoreAdapter::new(
            spi,
            cs,
            SpiAdapterConfig::new(FLASH_MAX_ADDRESS),
        ));
        let store_cfg = StoreConfig::new(KVS_MAGIC, KVS_MAX_HOPS).nonce(KVS_NONCE);
        let store = FlashStore::open(store_adapter, store_cfg, true).expect("store open failed");
        Self { store, wp }
    }
}
