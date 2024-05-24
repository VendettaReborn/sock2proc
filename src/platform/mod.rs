pub trait FindProc {
    fn resolve(
        src: Option<std::net::SocketAddr>,
        dst: Option<std::net::SocketAddr>,
        proto: i32,
    ) -> Option<String>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::FindProcImpl;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::FindProcImpl;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_compile() {
        let dst = std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)),
            80,
        );
        let _process_name = FindProcImpl::resolve(None, Some(dst), libc::IPPROTO_TCP);
    }
}
