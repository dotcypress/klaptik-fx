#![no_std]
#![no_main]

use defmt_rtt as _;

extern crate stm32g0xx_hal as hal;

#[cfg(feature = "probe")]
extern crate panic_probe;

#[cfg(not(feature = "probe"))]
extern crate panic_halt;

mod config;
mod controls;
mod display;
mod pins;
mod server;
mod store;

use config::*;
use controls::*;
use display::*;
use hal::{exti::*, gpio::*, i2c, prelude::*, spi, stm32, stm32::*, timer::*};
use klaptik::{drivers::fx::FxCommand, *};
use pins::*;
use server::*;
use store::*;

pub type Qei = hal::timer::qei::Qei<TIM1, (GpioB2, GpioB3)>;
pub type I2cDev = hal::i2c::I2c<I2C1, I2cSda, I2cClk>;
pub type SpiDev = hal::spi::Spi<SPI1, (SpiClk, SpiMiso, SpiMosi)>;

#[rtic::app(device = stm32, peripherals = true, dispatchers = [USART1, USART2])]
mod klaptik_fx {
    use super::*;

    #[shared]
    struct Shared {
        exti: EXTI,
        display: DisplayController,
        store: Store,
        gpio_a: Gpio,
        gpio_b: Gpio,
    }

    #[local]
    struct Local {
        encoder: Option<Encoder>,
        server: I2CServer,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(hal::rcc::Config::pll());
        let mut exti = ctx.device.EXTI;

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

        let spi = ctx.device.SPI1.spi(
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
            .I2C1
            .i2c(pins.i2c_sda, pins.i2c_clk, i2c_cfg, &mut rcc);
        i2c.listen(i2c::Event::AddressMatch);

        let server = I2CServer::new(i2c);
        let store = Store::new(spi.acquire(), pins.eeprom_cs).expect("store init failed");
        let display = DisplayController::new(
            spi.acquire(),
            pins.lcd_reset,
            pins.lcd_cs,
            pins.lcd_dc,
            lcd_backlight,
            &mut delay,
        );

        let gpio_a = Gpio::default();
        let gpio_b = Gpio::default();

        pins.gpio_a1.listen(SignalEdge::All, &mut exti);
        pins.gpio_a2.listen(SignalEdge::All, &mut exti);
        pins.gpio_b1.listen(SignalEdge::All, &mut exti);

        let encoder = if cfg!(feature = "qei") {
            let qei = delay.release().qei(
                (pins.gpio_b2.into_analog(), pins.gpio_b3.into_analog()),
                &mut rcc,
            );
            Some(Encoder::new(qei))
        } else {
            pins.gpio_b2.listen(SignalEdge::All, &mut exti);
            pins.gpio_b3.listen(SignalEdge::All, &mut exti);
            None
        };

        render::spawn(RenderRequest::new(Point::zero(), 0xff, 0)).expect("initial render failed");

        (
            Shared {
                gpio_a,
                gpio_b,
                display,
                exti,
                store,
            },
            Local { encoder, server },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI0_1)]
    fn gpio_a_edge(_: gpio_a_edge::Context) {
        gpio_event::spawn(Event::GPIO0).ok();
        gpio_event::spawn(Event::GPIO1).ok();
    }

    #[task(binds = EXTI2_3)]
    fn gpio_b1_edge(_: gpio_b1_edge::Context) {
        gpio_event::spawn(Event::GPIO2).ok();
        gpio_event::spawn(Event::GPIO3).ok();
    }

    #[task(binds = EXTI4_15)]
    fn gpio_b23_edge(_: gpio_b23_edge::Context) {
        gpio_event::spawn(Event::GPIO8).ok();
    }

    #[task(priority = 2, shared = [exti, gpio_a, gpio_b])]
    fn gpio_event(ctx: gpio_event::Context, ev: Event) {
        (ctx.shared.exti, ctx.shared.gpio_a, ctx.shared.gpio_b).lock(|exti, gpio_a, gpio_b| {
            for edge in [SignalEdge::Falling, SignalEdge::Rising] {
                if exti.is_pending(ev, edge) {
                    match ev {
                        Event::GPIO0 => gpio_a.record_edge(0, edge),
                        Event::GPIO1 => gpio_a.record_edge(1, edge),
                        Event::GPIO2 => gpio_b.record_edge(0, edge),
                        Event::GPIO8 => gpio_b.record_edge(1, edge),
                        Event::GPIO3 => gpio_b.record_edge(2, edge),
                        _ => unreachable!(),
                    };
                }
            }
            exti.unpend(ev);
        });
    }

    #[task(priority = 3, binds = I2C1, local = [server, encoder], shared = [display, gpio_a, gpio_b, store])]
    fn i2c_rx(ctx: i2c_rx::Context) {
        let i2c_rx::SharedResources {
            mut display,
            mut gpio_a,
            mut gpio_b,
            mut store,
        } = ctx.shared;
        let i2c_rx::LocalResources { server, encoder } = ctx.local;

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
                            0xfe => gpio_a.lock(|gpio_a| gpio_a.as_bytes()),
                            0xfd => gpio_b.lock(|gpio_b| gpio_b.as_bytes()),
                            0xfc => encoder
                                .as_ref()
                                .map(|enc| enc.as_bytes())
                                .unwrap_or([0xff, 0xff, 0xff, 0xff]),
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
