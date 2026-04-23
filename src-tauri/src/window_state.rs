use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PersistedWindowState {
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub maximized: bool,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

pub fn window_state_path(axagent_home: &Path) -> PathBuf {
    axagent_home.join("window-state.json")
}

pub fn load_window_state(axagent_home: &Path) -> Option<PersistedWindowState> {
    let path = window_state_path(axagent_home);
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn save_window_state(axagent_home: &Path, state: PersistedWindowState) -> io::Result<()> {
    std::fs::create_dir_all(axagent_home)?;
    let json = serde_json::to_vec_pretty(&state)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
    std::fs::write(window_state_path(axagent_home), json)
}

pub fn clamp_window_state_to_monitor(
    state: PersistedWindowState,
    monitor_width: f64,
    monitor_height: f64,
) -> PersistedWindowState {
    let max_width = monitor_width * 0.9;
    let max_height = monitor_height * 0.9;
    PersistedWindowState {
        width: state.width.clamp(640.0, max_width),
        height: state.height.clamp(480.0, max_height),
        maximized: state.maximized,
        fullscreen: state.fullscreen,
        x: state.x.map(|v| v.clamp(0.0, monitor_width - 100.0)),
        y: state.y.map(|v| v.clamp(0.0, monitor_height - 100.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_window_state_to_monitor, load_window_state, save_window_state, PersistedWindowState,
    };

    #[test]
    fn round_trips_window_state_in_axagent_home() {
        let test_dir = std::env::temp_dir().join(format!(
            "axagent-window-state-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        ));

        std::fs::create_dir_all(&test_dir).expect("failed to create temp dir");
        let state = PersistedWindowState {
            width: 1440.0,
            height: 960.0,
            maximized: true,
            fullscreen: false,
            x: Some(100.0),
            y: Some(50.0),
        };

        save_window_state(&test_dir, state).expect("failed to save window state");

        let restored = load_window_state(&test_dir).expect("failed to load saved window state");
        assert_eq!(restored, state);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn loads_legacy_state_without_new_fields() {
        let test_dir = std::env::temp_dir().join(format!(
            "axagent-window-state-legacy-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        ));

        std::fs::create_dir_all(&test_dir).expect("failed to create temp dir");
        let legacy_json = r#"{"width":1440.0,"height":960.0}"#;
        std::fs::write(test_dir.join("window-state.json"), legacy_json)
            .expect("failed to write legacy json");

        let restored = load_window_state(&test_dir).expect("failed to load legacy state");
        assert_eq!(restored.width, 1440.0);
        assert_eq!(restored.height, 960.0);
        assert!(!restored.maximized);
        assert!(!restored.fullscreen);
        assert!(restored.x.is_none());
        assert!(restored.y.is_none());

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn clamps_oversized_window_state_to_visible_monitor_bounds() {
        let clamped = clamp_window_state_to_monitor(
            PersistedWindowState {
                width: 2200.0,
                height: 1600.0,
                maximized: false,
                fullscreen: false,
                x: Some(2000.0),
                y: Some(1500.0),
            },
            1512.0,
            982.0,
        );

        assert!((clamped.width - 1360.8).abs() < f64::EPSILON);
        assert!((clamped.height - 883.8).abs() < 1e-9);
        // x clamped to monitor_width - 100
        assert!((clamped.x.unwrap() - 1412.0).abs() < f64::EPSILON);
        // y clamped to monitor_height - 100
        assert!((clamped.y.unwrap() - 882.0).abs() < f64::EPSILON);
    }
}
