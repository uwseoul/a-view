use std::sync::Mutex;

pub struct AppState {
    pub sleep_guard: Mutex<Option<keepawake::KeepAwake>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sleep_guard: Mutex::new(None),
        }
    }
}