#[cfg(target_os = "windows")]
fn main() {
    windows::build!(
        foundation::IReference,
        system::{KnownUserProperties, User, UserType},
    );
}

#[cfg(not(target_os = "windows"))]
fn main() {
    ()
}
