#![no_std]
#![no_main]

use defmt_rtt as _;

extern crate stm32g0xx_hal as hal;

#[cfg(feature = "probe")]
extern crate panic_probe;

#[cfg(not(feature = "probe"))]
extern crate panic_halt;

mod app;
mod display;
mod store;
mod wiring;

use app::*;
use display::*;
use hal::{exti::*, gpio::SignalEdge, i2c, prelude::*, spi, stm32, timer::*};
use klaptik::*;
use shared_bus_rtic::SharedBus;
use store::*;
use wiring::*;

#[rtic::app(device = stm32, peripherals = true, dispatchers = [CEC])]
mod klaptik_fx_app {
    use super::*;

    #[shared]
    struct Shared {
        app: App,
        display: SpriteDisplay<DisplayController, { SPRITES.len() }>,
        store: Store,
        i2c: I2cDev,
    }

    #[local]
    struct Local {
        exti: stm32::EXTI,
        ui: UI,
        ui_timer: Timer<stm32::TIM17>,
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

        let gpio = pins.gpio;
        gpio.gpio0.listen(SignalEdge::All, &mut exti);
        gpio.gpio2.listen(SignalEdge::All, &mut exti);

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let mut ui_timer = ctx.device.TIM17.timer(&mut rcc);
        ui_timer.start(350.millis());
        ui_timer.listen();

        let backlight_pwm = ctx.device.TIM14.pwm(100.kHz(), &mut rcc);
        let mut lcd_backlight = backlight_pwm.bind_pin(pins.lcd_backlight);
        lcd_backlight.enable();
        lcd_backlight.set_duty(0);

        let i2c = ctx.device.I2C2.i2c(
            pins.i2c_sda,
            pins.i2c_clk,
            i2c::Config::new(400.kHz()),
            &mut rcc,
        );

        let spi = ctx.device.SPI2.spi(
            (pins.spi_clk, pins.spi_miso, pins.spi_mosi),
            spi::MODE_0,
            16.MHz(),
            &mut rcc,
        );
        let spi_bus = shared_bus_rtic::new!(spi, SpiDev);

        let store = Store::new(spi_bus.acquire(), pins.eeprom_cs, pins.eeprom_wp);

        let display_ctrl = DisplayController::new(
            spi_bus.acquire(),
            pins.lcd_reset,
            pins.lcd_cs,
            pins.lcd_dc,
            lcd_backlight,
            &mut delay,
        );
        let mut display = SpriteDisplay::new(display_ctrl, SPRITES);

        let mut ui = UI::new();
        ui.render(&mut display);
        display.canvas().set_backlight(8);
        display.canvas().on();

        (
            Shared {
                display,
                store,
                i2c,
                app: App::new(),
            },
            Local { ui_timer, exti, ui },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM17, local = [ui, ui_timer], shared = [app, display, i2c, store])]
    fn ui_timer_tick(ctx: ui_timer_tick::Context) {
        let ui_timer_tick::LocalResources { ui, ui_timer } = ctx.local;
        let ui_timer_tick::SharedResources {
            mut app,
            mut display,
            i2c: _,
            store: _,
        } = ctx.shared;
        app.lock(|app| {
            app.animate();
            ui.update(app.state());
        });
        display.lock(|display| {
            ui.render(display);
        });
        ui_timer.clear_irq();
    }

    #[task(binds = EXTI0_1)]
    fn gpio_a_edge(_: gpio_a_edge::Context) {
        gpio_event::spawn(Event::GPIO0).ok();
    }

    #[task(binds = EXTI2_3)]
    fn gpio_b1_edge(_: gpio_b1_edge::Context) {
        gpio_event::spawn(Event::GPIO2).ok();
    }

    #[task(priority = 2, local = [exti])]
    fn gpio_event(ctx: gpio_event::Context, ev: Event) {
        ctx.local.exti.unpend(ev);
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
