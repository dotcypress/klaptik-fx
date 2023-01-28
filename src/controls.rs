use crate::*;
use hal::hal::Direction;

pub struct Controls {
    buttons: Buttons,
    encoder: Encoder,
}

impl Controls {
    pub fn new(qei: Qei) -> Self {
        Self {
            encoder: Encoder::new(qei),
            buttons: Buttons::default(),
        }
    }

    pub fn record_edge(&mut self, ev: Event, edge: SignalEdge) {
        self.buttons.record_edge(ev, edge);
    }

    pub fn buttons_state(&mut self) -> [u8; 4] {
        self.buttons.drain().as_bytes()
    }

    pub fn encoder_state(&mut self) -> [u8; 4] {
        self.encoder.as_bytes()
    }
}

pub struct Encoder {
    qei: Qei,
}

impl Encoder {
    pub fn new(qei: Qei) -> Self {
        Self { qei }
    }

    pub fn as_bytes(&self) -> [u8; 4] {
        let pulses = self.qei.count() / 2;
        [
            (pulses >> 8) as u8,
            pulses as u8,
            (self.qei.direction() == Direction::Downcounting) as u8,
            0,
        ]
    }
}

#[derive(Default)]
pub struct Buttons {
    falling_edges: [u8; 3],
    rising_edges: [u8; 3],
}

impl Buttons {
    pub fn drain(&mut self) -> Self {
        let snaphot = Self {
            falling_edges: self.falling_edges,
            rising_edges: self.rising_edges,
        };
        self.falling_edges = [0; 3];
        self.rising_edges = [0; 3];
        snaphot
    }

    pub fn record_edge(&mut self, ev: Event, edge: SignalEdge) {
        let idx = match ev {
            Event::GPIO0 => 0,
            Event::GPIO1 => 1,
            Event::GPIO2 => 2,
            _ => unreachable!(),
        };
        match edge {
            SignalEdge::Falling => self.falling_edges[idx] += 1,
            SignalEdge::Rising => self.rising_edges[idx] += 1,
            _ => unreachable!(),
        }
    }

    pub fn as_bytes(&self) -> [u8; 4] {
        let mut res = [0; 4];
        for (idx, b) in res.iter_mut().enumerate().take(3) {
            *b = self.falling_edges[idx] | self.rising_edges[idx] << 4;
        }
        res
    }
}
