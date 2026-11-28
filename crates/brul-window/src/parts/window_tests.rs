use std::sync::Arc;

use super::{Backend, WindowConfig, WindowError, WindowKey, WindowPosition};

#[test]
fn default_config_is_valid() {
    assert!(WindowConfig::default().validate().is_ok());
}

#[test]
fn the_default_backend_starts_without_a_gpu() {
    assert_eq!(WindowConfig::default().backend, Backend::Software);
}

#[test]
fn a_backend_parses_from_its_name() {
    assert_eq!("software".parse(), Ok(Backend::Software));
    assert_eq!("gpu".parse(), Ok(Backend::Gpu));
    assert!("other".parse::<Backend>().is_err());
}

#[test]
fn a_non_finite_position_is_rejected() {
    let config = WindowConfig {
        position: Some(WindowPosition {
            x: f64::INFINITY,
            y: 0.0,
        }),
        ..WindowConfig::default()
    };
    assert!(matches!(
        config.validate(),
        Err(WindowError::InvalidPosition)
    ));
}

#[test]
fn a_window_is_centered_on_its_monitor() {
    let monitor = super::PhysicalPosition::new(0, 0);
    let area = super::PhysicalSize::new(1920, 1080);
    let centered = super::centered_position(monitor, area, super::PhysicalSize::new(488, 800));
    assert_eq!((centered.x, centered.y), (716, 140));

    let second = super::PhysicalPosition::new(1920, 0);
    let centered = super::centered_position(second, area, super::PhysicalSize::new(488, 800));
    assert_eq!((centered.x, centered.y), (2636, 140));
}

#[test]
fn a_point_selects_the_monitor_it_falls_on() {
    let area = super::PhysicalSize::new(1920, 1080);
    let monitors = [
        (
            0,
            super::MonitorRect {
                origin: super::PhysicalPosition::new(0, 0),
                size: area,
            },
        ),
        (
            1,
            super::MonitorRect {
                origin: super::PhysicalPosition::new(1920, 0),
                size: area,
            },
        ),
    ];

    let pick = |x, y| {
        super::monitor_at_point(monitors.iter().copied(), super::PhysicalPosition::new(x, y))
    };
    assert_eq!(pick(100, 100), Some(0));
    assert_eq!(pick(2000, 100), Some(1));
    assert_eq!(pick(1920, 100), Some(1));
    assert_eq!(pick(-10, 500), Some(0));
    assert_eq!(
        super::monitor_at_point(
            std::iter::empty::<(u32, super::MonitorRect)>(),
            super::PhysicalPosition::new(0, 0),
        ),
        None
    );
}

#[test]
fn zero_size_is_rejected() {
    let config = WindowConfig {
        width: 0,
        ..WindowConfig::default()
    };
    assert!(matches!(config.validate(), Err(WindowError::EmptySize)));
}

#[test]
fn header_taller_than_window_is_rejected() {
    let config = WindowConfig {
        height: 40,
        header_height: 41,
        ..WindowConfig::default()
    };
    assert!(matches!(config.validate(), Err(WindowError::HeaderTooTall)));
}

#[test]
fn window_error_keeps_the_stage_names_apart() {
    let messages = [
        WindowError::EmptySize.to_string(),
        WindowError::HeaderTooTall.to_string(),
        WindowError::InvalidPosition.to_string(),
        WindowError::NoSurfaceConfig.to_string(),
        WindowError::NoContent.to_string(),
        WindowError::event_loop(std::io::Error::other("boom")).to_string(),
        WindowError::window(std::io::Error::other("boom")).to_string(),
        WindowError::gpu(std::io::Error::other("boom")).to_string(),
        WindowError::app(std::io::Error::other("boom")).to_string(),
    ];
    let mut unique = messages.to_vec();
    unique.sort();
    unique.dedup();

    assert_eq!(unique.len(), messages.len());
    assert_eq!(messages[5], "event loop: boom");
    assert_eq!(messages[6], "window creation: boom");
    assert_eq!(messages[7], "gpu: boom");
    assert_eq!(messages[8], "boom");
}

#[test]
fn a_failed_stage_keeps_its_cause() {
    let error = WindowError::gpu(std::io::Error::other("boom"));

    assert_eq!(error.to_string(), "gpu: boom");
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn an_undeclared_key_names_the_window() {
    assert_eq!(
        WindowError::Undeclared(WindowKey(7)).to_string(),
        "window 7 is not declared"
    );
}

#[test]
fn a_frameless_window_has_no_decorations_and_a_fixed_size() {
    let config = WindowConfig {
        width: 488,
        height: 800,
        transparent: true,
        ..WindowConfig::default()
    };

    let attributes = super::attributes(&config);

    assert!(!attributes.decorations);
    assert!(!attributes.resizable);
    #[cfg(windows)]
    assert!(!attributes.transparent);
    #[cfg(not(windows))]
    assert!(attributes.transparent);
    assert!(attributes.min_surface_size.is_some());
    assert!(attributes.max_surface_size.is_some());
}

#[test]
#[cfg(windows)]
fn a_gpu_window_keeps_winit_transparency() {
    let config = WindowConfig {
        transparent: true,
        backend: Backend::Gpu,
        ..WindowConfig::default()
    };

    assert!(super::attributes(&config).transparent);
}

#[test]
fn a_window_is_created_hidden_and_shown_with_its_first_frame() {
    let config = WindowConfig {
        visible: false,
        ..WindowConfig::default()
    };

    assert!(!super::attributes(&config).visible);
    assert!(!super::attributes(&WindowConfig::default()).visible);
}

#[test]
fn a_configured_icon_reaches_the_window_attributes() {
    let png = include_bytes!("../../../brul-gui/tests/fixtures/tint.png");
    let config = WindowConfig {
        icon: Some(Arc::from(&png[..])),
        ..WindowConfig::default()
    };

    assert!(super::attributes(&config).window_icon.is_some());
}

#[test]
fn a_decorated_window_is_resizable_without_a_size_cap() {
    let config = WindowConfig {
        title: "Settings".to_owned(),
        width: 760,
        height: 560,
        frameless: false,
        transparent: false,
        ..WindowConfig::default()
    };

    let attributes = super::attributes(&config);

    assert_eq!(attributes.title, "Settings");
    assert!(attributes.decorations);
    assert!(attributes.resizable);
    assert!(!attributes.transparent);
    assert!(attributes.min_surface_size.is_none());
    assert!(attributes.max_surface_size.is_none());
}

#[test]
fn header_hit_test_uses_physical_pixels() {
    let config = WindowConfig {
        header_height: 40,
        ..WindowConfig::default()
    };
    assert!(config.is_header(0.0, 1.0));
    assert!(config.is_header(39.9, 1.0));
    assert!(!config.is_header(40.0, 1.0));
    assert!(!config.is_header(-1.0, 1.0));
    assert!(config.is_header(79.0, 2.0));
    assert!(!config.is_header(80.0, 2.0));
}
