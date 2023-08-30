use crate::power::PowerState;

mod assets;
mod widgets;

pub use assets::*;
pub use widgets::*;

pub enum AppRequest {
    PowerOff,
}

pub enum AppEvent {
    AnimationFrame,
    Power(PowerState),
}

pub struct AppState {
    pub frame: usize,
}

pub struct App {
    state: AppState,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState { frame: 0 },
        }
    }

    pub fn state(&mut self) -> &AppState {
        &self.state
    }

    pub fn handle_event(&mut self, ev: AppEvent) -> Option<AppRequest> {
        match ev {
            AppEvent::AnimationFrame => {
                self.state.frame += 1;
                None
            }
            AppEvent::Power(_) => None,
        }
    }
}
