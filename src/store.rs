use crate::*;
use kvs::adapters::paged::PagedAdapter;
use kvs::adapters::spi::{SpiAdapterConfig, SpiStoreAdapter};
use kvs::adapters::StoreAdapter;
use kvs::*;
use shared_bus_rtic::SharedBus;
use uluru::LRUCache;

pub type StoreAdapterError = kvs::adapters::spi::Error<SharedBus<SpiDev>, EepromCS>;
pub type FlashStoreError = kvs::Error<StoreAdapterError>;

pub type FlashStore = KVStore<
    PagedAdapter<SpiStoreAdapter<SharedBus<SpiDev>, EepromCS, FLASH_ADDR_BYTES>, FLASH_PAGE_SIZE>,
    KVS_BUCKETS,
    KVS_SLOTS,
>;

pub type StoreResul<T> = Result<T, FlashStoreError>;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct SpriteInfo {
    pub glyphs: u8,
    pub glyph_size: Size,
}

impl SpriteInfo {
    pub fn glyph_len(&self) -> usize {
        self.glyph_size.width as usize * self.glyph_size.height as usize / 8
    }

    pub fn bitmap_len(&self) -> usize {
        self.glyph_len() * self.glyphs as usize
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Sprite {
    pub sprite_id: u8,
    pub addr: Address,
    pub info: SpriteInfo,
}

pub struct Store {
    fs: FlashStore,
    cache: LRUCache<Sprite, SPRITE_CACHE_SIZE>,
}

impl Store {
    pub fn new(spi_dev: SharedBus<SpiDev>, cs: EepromCS) -> StoreResul<Self> {
        let cfg = SpiAdapterConfig::new(FLASH_MAX_ADDRESS);
        let store_cfg = StoreConfig::new(KVS_MAGIC, KVS_MAX_HOPS).nonce(KVS_NONCE);
        let adapter = PagedAdapter::new(SpiStoreAdapter::new(spi_dev, cs, cfg));
        let fs = FlashStore::open(adapter, store_cfg, true)?;
        let cache = LRUCache::default();
        Ok(Self { fs, cache })
    }

    pub fn read(&mut self, addr: Address, buf: &mut [u8]) -> Result<(), StoreAdapterError> {
        self.fs.adapter().read(addr, buf)
    }

    pub fn read_register(&mut self, reg: u8) -> StoreResul<[u8; 4]> {
        let mut scratch = [0; 4];
        self.fs.load(&[b'm', reg], &mut scratch)?;
        Ok(scratch)
    }

    pub fn write_register(&mut self, reg: u8, val: [u8; 4]) -> StoreResul<()> {
        self.fs.insert(&[b'm', reg], &val)?;
        Ok(())
    }

    pub fn create_sprite(&mut self, sprite_id: SpriteId, info: SpriteInfo) -> StoreResul<()> {
        self.fs.alloc(&[b'b', sprite_id], info.bitmap_len(), None)?;
        self.fs.insert_val::<_, 8>(&[b's', sprite_id], &info)?;
        self.cache.clear();
        Ok(())
    }

    pub fn patch_sprite_bitmap(
        &mut self,
        sprite_id: SpriteId,
        offset: Address,
        patch: &[u8],
    ) -> StoreResul<()> {
        self.fs.patch(&[b'b', sprite_id], offset, patch)?;
        self.cache.clear();
        Ok(())
    }

    pub fn delete_sprite(&mut self, sprite_id: SpriteId) -> StoreResul<()> {
        self.cache.clear();
        self.fs.remove(&[b'b', sprite_id])?;
        self.fs.remove(&[b's', sprite_id])
    }

    pub fn get_sprite(&mut self, sprite_id: SpriteId) -> Option<Sprite> {
        match self.cache.find(|sprite| sprite.sprite_id == sprite_id) {
            Some(sprite) => Some(*sprite),
            None => match self.sprite_lookup(sprite_id) {
                Ok(Some(sprite)) => {
                    self.cache.insert(sprite);
                    Some(sprite)
                }
                _ => None,
            },
        }
    }

    fn sprite_lookup(&mut self, sprite_id: SpriteId) -> StoreResul<Option<Sprite>> {
        match self.fs.load_val::<_, 8>(&[b's', sprite_id]) {
            Ok(info) => {
                let bucket = self.fs.lookup(&[b'b', sprite_id])?;
                let cache = Sprite {
                    sprite_id,
                    info,
                    addr: bucket.val_address(),
                };
                Ok(Some(cache))
            }
            Err(kvs::Error::KeyNotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
