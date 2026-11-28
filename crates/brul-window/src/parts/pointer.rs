use super::PhysicalPosition;

pub(super) fn pointer_position() -> Option<PhysicalPosition<i32>> {
    platform_pointer()
}

#[cfg(windows)]
fn platform_pointer() -> Option<PhysicalPosition<i32>> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some(PhysicalPosition::new(point.x, point.y))
}

#[cfg(target_os = "macos")]
fn platform_pointer() -> Option<PhysicalPosition<i32>> {
    use objc2_core_graphics::CGEvent;

    let event = CGEvent::new(None)?;
    let point = CGEvent::location(Some(&event));
    Some(PhysicalPosition::new(
        point.x.round() as i32,
        point.y.round() as i32,
    ))
}

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
fn platform_pointer() -> Option<PhysicalPosition<i32>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::ConnectionExt as _;

    let (connection, screen) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots.get(screen)?.root;
    let reply = connection.query_pointer(root).ok()?.reply().ok()?;
    Some(PhysicalPosition::new(
        i32::from(reply.root_x),
        i32::from(reply.root_y),
    ))
}

#[cfg(not(any(
    windows,
    target_os = "macos",
    all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )
)))]
fn platform_pointer() -> Option<PhysicalPosition<i32>> {
    None
}
