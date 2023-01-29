#![no_std]
#![no_main]

use defmt_rtt as _;

extern crate stm32c0xx_hal as hal;
// extern crate stm32g0xx_hal as hal;

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
pub type I2cDev = hal::i2c::I2c<I2C, I2cSda, I2cClk>;
pub type SpiDev = hal::spi::Spi<SPI, (SpiClk, SpiMiso, SpiMosi)>;

#[rtic::app(device = stm32, peripherals = true, dispatchers = [USART1, USART2])]
mod klaptik_fx {
    use super::*;

    #[shared]
    struct Shared {
        exti: EXTI,
        display: DisplayController,
        store: Store,
        controls: Controls,
    }

    #[local]
    struct Local {
        server: I2CServer,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(hal::rcc::Config::hsi(hal::rcc::Prescaler::NotDivided));
        let mut exti = ctx.device.EXTI;

        let pins = Pins::new(
            ctx.device.GPIOA,
            ctx.device.GPIOB,
            ctx.device.GPIOC,
            &mut rcc,
        );
        pins.gpio_a1.listen(SignalEdge::All, &mut exti);
        pins.gpio_a2.listen(SignalEdge::All, &mut exti);
        pins.gpio_b1.listen(SignalEdge::All, &mut exti);

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let backlight_pwm = ctx.device.TIM14.pwm(100.kHz(), &mut rcc);
        let mut lcd_backlight = backlight_pwm.bind_pin(pins.lcd_backlight);
        lcd_backlight.enable();
        lcd_backlight.set_duty(0);

        let spi = ctx.device.SPI.spi(
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
            .I2C
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

        let controls = Controls::new(delay.release().qei(
            (pins.gpio_b2.into_analog(), pins.gpio_b3.into_analog()),
            &mut rcc,
        ));

        defmt::info!("init complated");
        (
            Shared {
                controls,
                display,
                exti,
                store,
            },
            Local { server },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI0_1)]
    fn gpio_a_edge(_: gpio_a_edge::Context) {
        gpio_event::spawn(Event::GPIO0).ok();
        gpio_event::spawn(Event::GPIO1).ok();
    }

    #[task(binds = EXTI2_3)]
    fn gpio_b_edge(_: gpio_b_edge::Context) {
        gpio_event::spawn(Event::GPIO2).ok();
    }

    #[task(priority = 3, binds = I2C, local = [server], shared = [display, controls, store])]
    fn i2c_rx(ctx: i2c_rx::Context) {
        let i2c_rx::SharedResources {
            mut display,
            mut controls,
            mut store,
        } = ctx.shared;
        let server = ctx.local.server;

        loop {
            match server.poll() {
                Ok(None) => break,
                Ok(Some(req)) => match req {
                    Request::Render(req) => render::spawn(req).expect("render failed"),
                    Request::ReadRegister(reg) => {
                        server.set_response(match reg {
                            0xff => display.lock(|disp| disp.config()),
                            0xfe => controls.lock(|ctrl| ctrl.buttons_state()),
                            0xfd => controls.lock(|ctrl| ctrl.encoder_state()),
                            reg => store
                                .lock(|store| store.read_register(reg))
                                .unwrap_or([0xff, 0xff, 0xff, 0xff]),
                        });
                    }
                    Request::WriteRegister(reg, val) => match reg {
                        0xff => display.lock(|disp| disp.set_config(val)),
                        reg if reg < 0xfd => {
                            store
                                .lock(|store| store.write_register(reg, val))
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
                },
                Err(_) => {
                    defmt::error!("poll failed");
                    server.reset();
                }
            }
        }
    }

    #[task(priority = 2, shared = [exti, controls])]
    fn gpio_event(ctx: gpio_event::Context, ev: Event) {
        (ctx.shared.exti, ctx.shared.controls).lock(|exti, control| {
            for edge in [SignalEdge::Falling, SignalEdge::Rising] {
                if exti.is_pending(ev, edge) {
                    control.record_edge(ev, edge);
                }
            }
            exti.unpend(ev);
        });
    }

    #[task(capacity = 64, shared = [display, store])]
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
