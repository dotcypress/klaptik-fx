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

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn animate(&mut self) {
        self.state.frame += 1;
    }
}
