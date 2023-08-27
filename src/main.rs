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
mod ui;
mod wiring;

use app::*;
use display::*;
use hal::{exti::*, gpio::SignalEdge, i2c, prelude::*, spi, stm32, stm32::*, timer::*};
use klaptik::*;
use kvs::adapters::paged::PagedAdapter;
use kvs::adapters::spi::*;
use kvs::*;
use shared_bus_rtic::SharedBus;
use store::*;
use ui::*;
use wiring::*;

#[rtic::app(device = stm32, peripherals = true, dispatchers = [CEC])]
mod klaptik_fx {
    use super::*;

    #[shared]
    struct Shared {
        app: App,
        display: SpriteDisplay<DisplayController, 3>,
    }

    #[local]
    struct Local {
        exti: EXTI,
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

        pins.gpio.gpio0.listen(SignalEdge::All, &mut exti);
        pins.gpio.gpio2.listen(SignalEdge::All, &mut exti);

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
        let spi_bus = shared_bus_rtic::new!(spi, SpiDev);

        let store_adapter = FlashStoreAdapter::new(SpiStoreAdapter::new(
            spi_bus.acquire(),
            pins.eeprom_cs,
            SpiAdapterConfig::new(FLASH_MAX_ADDRESS),
        ));
        let store_cfg = StoreConfig::new(KVS_MAGIC, KVS_MAX_HOPS).nonce(KVS_NONCE);
        let _store = FlashStore::open(store_adapter, store_cfg, true).expect("store open failed");

        let _i2c: I2cDev = ctx.device.I2C2.i2c(
            pins.i2c_sda,
            pins.i2c_clk,
            i2c::Config::new(400.kHz()),
            &mut rcc,
        );

        let mut delay = ctx.device.TIM1.delay(&mut rcc);
        let mut display_ctrl = DisplayController::new(
            spi_bus.acquire(),
            pins.lcd_reset,
            pins.lcd_cs,
            pins.lcd_dc,
            lcd_backlight,
            &mut delay,
        );
        display_ctrl.set_backlight(10);
        display_ctrl.on();

        let display = SpriteDisplay::new(display_ctrl, SPRITES);

        let app = App::new();
        let ui = UI::new();

        let mut ui_timer = ctx.device.TIM17.timer(&mut rcc);
        ui_timer.start(350.millis());
        ui_timer.listen();

        (
            Shared { app, display },
            Local { ui_timer, exti, ui },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM17, local = [ui, ui_timer], shared = [app, display])]
    fn ui_timer_tick(ctx: ui_timer_tick::Context) {
        let ui_timer_tick::LocalResources { ui, ui_timer } = ctx.local;
        let ui_timer_tick::SharedResources {
            mut app,
            mut display,
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
