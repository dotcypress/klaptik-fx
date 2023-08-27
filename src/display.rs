use crate::*;
use hal::timer::delay::Delay;
use hal::timer::pwm::PwmPin;
use klaptik::drivers::st7567::*;
use klaptik::Point;
use shared_bus_rtic::SharedBus;

pub type Backlight = PwmPin<TIM14, Channel1>;
pub type DisplayDriver = ST7567<SharedBus<SpiDev>, LcdReset, LcdCS, LcdDC>;

pub struct DisplayController {
    backlight: Backlight,
    canvas: DisplayDriver,
}

impl DisplayController {
    pub fn new(
        spi: SharedBus<SpiDev>,
        lcd_reset: LcdReset,
        lcd_cs: LcdCS,
        lcd_dc: LcdDC,
        backlight: Backlight,
        delay: &mut Delay<TIM1>,
    ) -> Self {
        let mut canvas = ST7567::new(spi, lcd_cs, lcd_dc, lcd_reset);
        canvas.set_offset(Point::new(4, 0));
        canvas.reset(delay);
        canvas
            .link()
            .command(|tx| tx.write(&[Command::SegmentDirectionRev as _]))
            .ok();
        Self { backlight, canvas }
    }

    pub fn set_backlight(&mut self, level: u8) {
        let backlight = level.clamp(0, 15) as u32;
        let duty = backlight * self.backlight.get_max_duty() as u32 / 16;
        self.backlight.set_duty(duty as _);
    }

    pub fn on(&mut self) {
        self.canvas.on();
    }

    pub fn off(&mut self) {
        self.canvas.off();
    }

    pub fn canvas(&mut self) -> &mut DisplayDriver {
        &mut self.canvas
    }
}

impl Canvas for DisplayController {
    fn draw(&mut self, bounds: Rectangle, bitmap: &[u8]) {
        self.canvas.draw(bounds, bitmap);
    }
}
