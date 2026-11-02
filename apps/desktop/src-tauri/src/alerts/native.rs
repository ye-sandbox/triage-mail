//! Linux notification service. No shell commands or remote assets are involved.
use triage_ai::alerts::DeliveryOutcome;
#[cfg(target_os = "linux")]
async fn connection() -> Result<zbus::Connection, zbus::Error> {
    zbus::Connection::session().await
}
#[cfg(target_os = "linux")]
pub async fn available() -> bool {
    let Ok(c) = connection().await else {
        return false;
    };
    let Ok(p) = zbus::Proxy::new(
        &c,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await
    else {
        return false;
    };
    p.call::<_, _, Vec<String>>("GetCapabilities", &())
        .await
        .is_ok()
}
#[cfg(target_os = "linux")]
pub async fn notify() -> DeliveryOutcome {
    let Ok(c) = connection().await else {
        return DeliveryOutcome::Unavailable;
    };
    let Ok(p) = zbus::Proxy::new(
        &c,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await
    else {
        return DeliveryOutcome::Unavailable;
    };
    let hints = std::collections::HashMap::<&str, zbus::zvariant::Value<'_>>::new();
    let result = p
        .call::<_, _, u32>(
            "Notify",
            &(
                "triage-mail",
                0u32,
                "",
                "New mail",
                "A new message matched an alert rule.",
                Vec::<String>::new(),
                hints,
                -1i32,
            ),
        )
        .await;
    if result.is_ok() {
        DeliveryOutcome::Accepted
    } else {
        DeliveryOutcome::Unavailable
    }
}
#[cfg(not(target_os = "linux"))]
pub async fn available() -> bool {
    false
}
#[cfg(not(target_os = "linux"))]
pub async fn notify() -> DeliveryOutcome {
    DeliveryOutcome::Unavailable
}
