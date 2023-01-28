use crate::*;
use i2c::I2cDirection::*;
use kvs::Address;

pub type Error = hal::i2c::Error;

pub enum Request {
    Render(RenderRequest),
    ReadRegister(u8),
    WriteRegister(u8, [u8; 4]),
    CreateSprite(SpriteId, SpriteInfo),
    PatchSprite(SpriteId, Address),
    DeleteSprite(SpriteId),
}

pub enum Mode {
    Command,
    Waiting(FxCommand, u8),
    Upload(SpriteId, SpriteInfo, usize),
}

impl Mode {
    pub fn packet_len(&self) -> usize {
        match self {
            Mode::Command => 2,
            Mode::Waiting(_, _) => 4,
            Mode::Upload(_, info, sent) => usize::min(255, info.bitmap_len() - sent),
        }
    }
}

pub struct I2CServer {
    i2c: I2cDev,
    mode: Mode,
    response: [u8; 4],
    payload: [u8; 255],
    payload_len: usize,
}

impl I2CServer {
    pub fn new(i2c: I2cDev) -> Self {
        Self {
            i2c,
            mode: Mode::Command,
            response: [0; 4],
            payload: [0; 255],
            payload_len: 2,
        }
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Command;
        self.response = [0; 4];
        self.payload = [0; 255];
        self.payload_len = 2;
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.payload[..self.payload_len]
    }

    pub fn set_response(&mut self, res: [u8; 4]) {
        self.response = res;
    }

    pub fn poll(&mut self) -> Result<Option<Request>, Error> {
        loop {
            let packet_len = self.mode.packet_len();
            self.payload_len = packet_len;

            match self.i2c.slave_addressed()? {
                Some((addr, MasterReadSlaveWrite)) if addr == FX_ADDRESS => {
                    let resp = self.response;
                    self.response = [0; 4];
                    self.i2c.slave_sbc(true);
                    self.i2c.slave_write(&resp)?;
                }
                Some((addr, MasterWriteSlaveRead)) => match addr {
                    FX_ADDRESS => {
                        self.i2c.slave_sbc(false);
                        self.i2c.slave_read(&mut self.payload[..packet_len])?;
                        let req = match self.mode {
                            Mode::Command => {
                                let arg = self.payload[1];
                                match self.payload[0] {
                                    0x00 => Request::ReadRegister(arg),
                                    0x80 => {
                                        self.mode = Mode::Waiting(FxCommand::WriteRegister, arg);
                                        continue;
                                    }
                                    0x81 => {
                                        self.mode = Mode::Waiting(FxCommand::UploadSprite, arg);
                                        continue;
                                    }
                                    0x82 => {
                                        self.mode = Mode::Waiting(FxCommand::DeleteSprite, arg);
                                        continue;
                                    }
                                    _ => continue,
                                }
                            }
                            Mode::Waiting(FxCommand::WriteRegister, reg) => {
                                self.mode = Mode::Command;
                                let mut val = [0; 4];
                                val.copy_from_slice(&self.payload[..4]);
                                Request::WriteRegister(reg, val)
                            }
                            Mode::Waiting(FxCommand::UploadSprite, sprite_id) => {
                                if self.payload[0] != sprite_id {
                                    self.mode = Mode::Command;
                                    continue;
                                }
                                let info = SpriteInfo {
                                    glyphs: self.payload[3],
                                    glyph_size: Size::new(self.payload[1], self.payload[2]),
                                };
                                self.mode = Mode::Upload(sprite_id, info, 0);
                                Request::CreateSprite(sprite_id, info)
                            }
                            Mode::Waiting(FxCommand::DeleteSprite, sprite_id) => {
                                if self.payload[0] != sprite_id || &self.payload[1..4] != b"del" {
                                    self.mode = Mode::Command;
                                    continue;
                                }
                                Request::DeleteSprite(sprite_id)
                            }
                            Mode::Upload(sprite_id, info, sent) => {
                                self.mode = if info.bitmap_len() > sent + packet_len {
                                    Mode::Upload(sprite_id, info, sent + packet_len)
                                } else {
                                    Mode::Command
                                };
                                Request::PatchSprite(sprite_id, sent)
                            }
                            _ => {
                                self.mode = Mode::Command;
                                continue;
                            }
                        };
                        return Ok(Some(req));
                    }
                    RENDER_ADDRESS => {
                        self.i2c.slave_sbc(false);
                        self.i2c.slave_read(&mut self.payload[..4])?;
                        let render_req = RenderRequest::from_bytes(&self.payload[..4]);
                        return Ok(Some(Request::Render(render_req)));
                    }
                    _ => {}
                },
                _ => {
                    self.i2c.clear_irq(i2c::Event::AddressMatch);
                    return Ok(None);
                }
            }
        }
    }
}
