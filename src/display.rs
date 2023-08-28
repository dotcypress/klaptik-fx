use crate::*;
use drivers::st7567::Command;

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
        delay: &mut DisplayDelay,
    ) -> Self {
        let mut canvas = DisplayDriver::new(spi, lcd_cs, lcd_dc, lcd_reset);
        canvas.set_offset(klaptik::Point::new(4, 0));
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

impl klaptik::Canvas for DisplayController {
    fn draw(&mut self, bounds: klaptik::Rectangle, bitmap: &[u8]) {
        self.canvas.draw(bounds, bitmap);
    }
}
