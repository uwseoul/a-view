use std::sync::Mutex;

pub struct AppState {
    pub sleep_guard: Mutex<Option<keepawake::KeepAwake>>,
    pub sleep_started_at: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sleep_guard: Mutex::new(None),
            sleep_started_at: Mutex::new(None),
        }
    }
}