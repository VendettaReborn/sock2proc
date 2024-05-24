pub(crate) fn check(src: Option<std::net::SocketAddr>, dst: Option<std::net::SocketAddr>) -> bool {
    if src.is_none() && dst.is_none() {
        false
    } else if src.is_some() && dst.is_some() {
        let inner1 = src.unwrap();
        let inner2 = dst.unwrap();
        (inner1.is_ipv4() && inner2.is_ipv6()) || (inner2.is_ipv4() && inner1.is_ipv6())
    } else {
        true
    }
}

pub(crate) fn is_ipv6(src: Option<std::net::SocketAddr>, dst: Option<std::net::SocketAddr>) -> bool {
    if src.is_some() {
        src.unwrap().is_ipv6()
    } else {
        dst.unwrap().is_ipv6()
    }
}