use crate::*;
use hal::timer::delay::Delay;
use hal::timer::pwm::PwmPin;
use klaptik::drivers::st7567;
use klaptik::Point;
use shared_bus_rtic::SharedBus;

pub type DisplayDriver = st7567::ST7567<SharedBus<SpiDev>, LcdReset, LcdCS, LcdDC>;
pub type Backlight = PwmPin<TIM14, Channel1>;

pub struct DisplayController {
    backlight_pwm: Backlight,
    canvas: DisplayDriver,
}

impl DisplayController {
    pub fn new(
        spi: SharedBus<SpiDev>,
        lcd_reset: LcdReset,
        lcd_cs: LcdCS,
        lcd_dc: LcdDC,
        backlight_pwm: Backlight,
        delay: &mut Delay<TIM1>,
    ) -> Self {
        let mut canvas = st7567::ST7567::new(spi, lcd_cs, lcd_dc, lcd_reset);
        canvas.set_offset(Point::new(4, 0));
        canvas.reset(delay);
        canvas
            .link()
            .command(|tx| tx.write(&[st7567::Command::SegmentDirectionRev as _]))
            .ok();
        Self {
            backlight_pwm,
            canvas,
        }
    }

    pub fn set_config(&mut self, on: bool, backlight: u8) {
        if on {
            self.canvas.on();
        } else {
            self.canvas.off();
        }

        let backlight = backlight.clamp(0, 15) as u32;
        let duty = backlight * self.backlight_pwm.get_max_duty() as u32 / 16;
        self.backlight_pwm.set_duty(duty as _);
    }
}

impl Canvas for DisplayController {
    fn draw(&mut self, bounds: Rectangle, bitmap: &[u8]) {
        self.canvas.draw(bounds, bitmap);
    }
}
