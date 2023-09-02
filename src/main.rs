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
mod power;
mod store;
mod wiring;

use app::*;
use display::*;
use hal::{analog::adc::*, exti::*, gpio::SignalEdge, i2c, prelude::*, spi, stm32, timer::*};
use klaptik::*;
use power::*;
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
        exti: stm32::EXTI,
        store: Store,
        power: PowerController,
    }

    #[local]
    struct Local {
        ui: UI,
        render_timer: Timer<stm32::TIM17>,
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

        let mut delay = ctx.device.TIM1.delay(&mut rcc);

        let mut render_timer = ctx.device.TIM17.timer(&mut rcc);
        render_timer.start(350.millis());
        render_timer.listen();

        let backlight_pwm = ctx.device.TIM14.pwm(100.kHz(), &mut rcc);
        let mut lcd_backlight = backlight_pwm.bind_pin(pins.lcd_backlight);
        lcd_backlight.enable();
        lcd_backlight.set_duty(0);

        let mut adc = ctx.device.ADC.constrain(&mut rcc);
        adc.set_sample_time(SampleTime::T_80);
        adc.set_precision(Precision::B_12);
        adc.set_oversampling_ratio(OversamplingRatio::X_16);
        adc.set_oversampling_shift(16);
        adc.oversampling_enable(true);
        delay.delay(100.micros());
        adc.calibrate();

        let i2c = ctx.device.I2C2.i2c(
            pins.i2c_sda,
            pins.i2c_clk,
            i2c::Config::new(400.kHz()),
            &mut rcc,
        );
        let i2c_bus = shared_bus_rtic::new!(i2c, I2cDev);
        let charger = Charger::new(i2c_bus.acquire());
        let mut power = PowerController::new(
            adc,
            pins.vcc_sense,
            charger,
            pins.power_en,
            pins.power_fault.listen(SignalEdge::Falling, &mut exti),
        );
        power.power_on();
        delay.delay(10.millis());
        pins.power_int.listen(SignalEdge::Falling, &mut exti);

        let spi = ctx.device.SPI2.spi(
            (pins.spi_clk, pins.spi_miso, pins.spi_mosi),
            spi::MODE_0,
            16.MHz(),
            &mut rcc,
        );
        let spi_bus = shared_bus_rtic::new!(spi, SpiDev);
        let display_ctrl = DisplayController::new(
            spi_bus.acquire(),
            pins.lcd_reset,
            pins.lcd_cs,
            pins.lcd_dc,
            lcd_backlight,
            &mut delay,
        );
        let mut display = SpriteDisplay::new(display_ctrl, SPRITES);

        let mut store = Store::new(spi_bus.acquire(), pins.eeprom_cs, pins.eeprom_wp);
        if store.store.exists(b"aaa").unwrap() {
            defmt::info!("Store ok");
        } else {
            defmt::info!("Store empty");
            store.store.insert(b"aaa", b"aaa").unwrap();
            defmt::info!("Store updated");
        }

        let app = App::new();
        let mut ui = UI::new();
        ui.render(&mut display);
        display.canvas().set_backlight(4);
        display.canvas().switch_on();

        exti.wakeup(Event::GPIO5);

        defmt::info!("Starting app");

        (
            Shared {
                app,
                power,
                display,
                store,
                exti,
            },
            Local { render_timer, ui },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI0_1, shared = [exti])]
    fn gpio_edge(ctx: gpio_edge::Context) {
        let gpio_edge::SharedResources { mut exti } = ctx.shared;
        if exti.lock(|exti| exti.is_pending(Event::GPIO0, SignalEdge::Falling)) {
            defmt::info!("GPIO interrupt");
            exti.lock(|exti| exti.unpend(Event::GPIO0));
        }
    }

    #[task(binds = EXTI4_15, shared = [power, store, exti])]
    fn power_int(ctx: power_int::Context) {
        let power_int::SharedResources {
            mut exti,
            mut power,
            store: _,
        } = ctx.shared;

        if exti.lock(|exti| exti.is_pending(Event::GPIO14, SignalEdge::Falling)) {
            defmt::info!("Overcurrent");
            exti.lock(|exti| exti.unpend(Event::GPIO14));
        }

        if exti.lock(|exti| exti.is_pending(Event::GPIO5, SignalEdge::Falling)) {
            defmt::info!("Changer interrupt");
            let _ = power.lock(|power| power.state());
            exti.lock(|exti| exti.unpend(Event::GPIO5));
        }
    }

    #[task(binds = TIM17, local = [ui, render_timer], shared = [app, display])]
    fn render(ctx: render::Context) {
        let render::LocalResources { ui, render_timer } = ctx.local;
        let render::SharedResources {
            mut app,
            mut display,
        } = ctx.shared;
        app.lock(|app| ui.update(app.state()));
        display.lock(|display| ui.render(display));
        render_timer.clear_irq();
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
