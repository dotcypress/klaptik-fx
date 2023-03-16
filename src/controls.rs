use crate::*;
use hal::hal::Direction;

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
pub struct Gpio {
    falling_edges: [u8; 4],
    rising_edges: [u8; 4],
}

impl Gpio {
    pub fn record_edge(&mut self, idx: usize, edge: SignalEdge) {
        match edge {
            SignalEdge::Falling => self.falling_edges[idx] += 1,
            SignalEdge::Rising => self.rising_edges[idx] += 1,
            _ => unreachable!(),
        }
    }

    pub fn as_bytes(&mut self) -> [u8; 4] {
        let mut res = [0; 4];
        for (idx, b) in res.iter_mut().enumerate() {
            *b = self.falling_edges[idx] | self.rising_edges[idx] << 4;
        }
        self.falling_edges = [0; 4];
        self.rising_edges = [0; 4];
        res
    }
}
