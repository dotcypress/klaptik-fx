#![no_std]
#![no_main]

use defmt_rtt as _;

extern crate stm32g0xx_hal as hal;

#[cfg(feature = "probe")]
extern crate panic_probe;

#[cfg(not(feature = "probe"))]
extern crate panic_halt;

mod config;
mod display;
mod pins;
mod server;
mod store;

use config::*;
use display::*;
use hal::{exti::*, gpio::*, i2c, prelude::*, spi, stm32, stm32::*, timer::*};
use klaptik::{drivers::fx::FxCommand, *};
use pins::*;
use server::*;
use store::*;

#[rtic::app(device = stm32, peripherals = true, dispatchers = [USART1, USART2])]
mod klaptik_fx {
    use super::*;

    #[shared]
    struct Shared {
        exti: EXTI,
        display: DisplayController,
        store: Store,
    }

    #[local]
    struct Local {
        server: I2CServer,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(hal::rcc::Config::pll());
        let exti = ctx.device.EXTI;

        let pins = Pins::new(
            ctx.device.GPIOA,
            ctx.device.GPIOB,
            ctx.device.GPIOC,
            &mut rcc,
        );

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let backlight_pwm = ctx.device.TIM14.pwm(100.kHz(), &mut rcc);
        let mut lcd_backlight = backlight_pwm.bind_pin(pins.lcd_backlight);
        lcd_backlight.enable();
        lcd_backlight.set_duty(0);

        let spi = ctx.device.SPI2.spi(
            (pins.spi_clk, pins.spi_miso, pins.spi_mosi),
            spi::MODE_0,
            16.MHz(),
            &mut rcc,
        );
        let spi = shared_bus_rtic::new!(spi, SpiDev);

        let mut i2c_cfg = i2c::Config::new(400.kHz());
        i2c_cfg.slave_address_2(0x2a, i2c::SlaveAddressMask::MaskOneBit);
        let mut i2c = ctx
            .device
            .I2C2
            .i2c(pins.i2c_sda, pins.i2c_clk, i2c_cfg, &mut rcc);
        i2c.listen(i2c::Event::AddressMatch);

        let server = I2CServer::new(i2c);
        let store =
            Store::new(spi.acquire(), pins.eeprom_cs, pins.eeprom_wp).expect("store init failed");
        let display = DisplayController::new(
            spi.acquire(),
            pins.lcd_reset,
            pins.lcd_cs,
            pins.lcd_dc,
            lcd_backlight,
            &mut delay,
        );

        render::spawn(RenderRequest::new(Point::zero(), 0xff, 0)).expect("initial render failed");

        (
            Shared {
                display,
                exti,
                store,
            },
            Local { server },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI0_1, shared = [exti])]
    fn gpio_0_1_edge(_: gpio_0_1_edge::Context) {
        gpio_event::spawn(Event::GPIO0).ok();
    }

    #[task(binds = EXTI2_3)]
    fn gpio_2_3_edge(_: gpio_2_3_edge::Context) {
        gpio_event::spawn(Event::GPIO2).ok();
    }

    #[task(binds = EXTI4_15)]
    fn gpio_4_15_edge(_: gpio_4_15_edge::Context) {
        gpio_event::spawn(Event::GPIO4).ok();
    }

    #[task(priority = 2, shared = [exti])]
    fn gpio_event(mut ctx: gpio_event::Context, ev: Event) {
        ctx.shared.exti.lock(|exti| {
            for edge in [SignalEdge::Falling, SignalEdge::Rising] {
                if exti.is_pending(ev, edge) {
                    match ev {
                        Event::GPIO0 => todo!(),
                        Event::GPIO1 => todo!(),
                        Event::GPIO2 => todo!(),
                        Event::GPIO3 => todo!(),
                        Event::GPIO4 => todo!(),
                        Event::GPIO5 => todo!(),
                        Event::GPIO6 => todo!(),
                        Event::GPIO7 => todo!(),
                        _ => unreachable!(),
                    };
                }
            }
            exti.unpend(ev);
        });
    }

    #[task(priority = 3, binds = I2C1, local = [server], shared = [display, store])]
    fn i2c_rx(ctx: i2c_rx::Context) {
        let i2c_rx::SharedResources {
            mut display,
            mut store,
        } = ctx.shared;
        let i2c_rx::LocalResources { server } = ctx.local;

        loop {
            match server.poll() {
                Ok(None) => break,
                Ok(Some(req)) => match req {
                    Request::Render(req) => render::spawn(req).expect("render failed"),
                    Request::ReadRegister(reg) => {
                        server.set_response(match reg {
                            0xff => {
                                let mut state = display.lock(|display| display.config());
                                state[3] =
                                    store.lock(|store| store.get_sprites_count()).unwrap_or(0);
                                state
                            }
                            reg => store
                                .lock(|store| store.read_nvm(reg))
                                .unwrap_or([0xff, 0xff, 0xff, 0xff]),
                        });
                    }
                    Request::WriteRegister(reg, val) => match reg {
                        0xff => display.lock(|disp| disp.set_config(val)),
                        reg if reg < 0xfd => {
                            store
                                .lock(|store| store.write_nvm(reg, val))
                                .unwrap_or_else(|_| defmt::error!("reg write failed"));
                        }
                        _ => {}
                    },
                    Request::CreateSprite(sprite_id, info) => store
                        .lock(|store| store.create_sprite(sprite_id, info))
                        .unwrap_or_else(|_| defmt::error!("create sprite failed")),
                    Request::PatchSprite(sprite_id, offset) => store
                        .lock(|store| {
                            store.patch_sprite_bitmap(sprite_id, offset, server.get_payload())
                        })
                        .unwrap_or_else(|_| defmt::error!("patch sprite failed")),
                    Request::DeleteSprite(sprite_id) => store
                        .lock(|store| store.delete_sprite(sprite_id))
                        .unwrap_or_else(|_| defmt::error!("delete sprite failed")),
                    Request::DeleteAllSprites => store
                        .lock(|store| store.delete_all_sprites())
                        .unwrap_or_else(|_| defmt::error!("delete all sprites failed")),
                },
                Err(_) => {
                    defmt::error!("poll failed");
                    server.reset();
                }
            }
        }
    }

    #[task(capacity = 128, shared = [display, store])]
    fn render(ctx: render::Context, req: RenderRequest) {
        let render::SharedResources {
            mut display,
            mut store,
        } = ctx.shared;

        if let Some(sprite) = store.lock(|store| store.get_sprite(req.sprite_id)) {
            let glyph_len = sprite.info.glyph_len();
            let mut frame_buffer = [0; 1024];
            if req.glyph >= sprite.info.glyphs || sprite.info.glyph_len() > frame_buffer.len() {
                return;
            }
            let addr = sprite.addr + glyph_len * req.glyph as usize;
            if store
                .lock(|store| store.read(addr, &mut frame_buffer[..glyph_len]))
                .is_ok()
            {
                let bounds = Rectangle::new(req.origin, sprite.info.glyph_size);
                display.lock(|disp| disp.draw(bounds, &frame_buffer[..glyph_len]));
            }
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            if cfg!(feature = "probe") {
                rtic::export::nop();
            } else {
                rtic::export::wfi();
            }
        }
    }
}
