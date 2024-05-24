// SPDX-License-Identifier: MIT

use std::{
    fs, io,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::Path,
};

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_DUMP, NLM_F_REQUEST,
};
use netlink_packet_sock_diag::{
    constants::*,
    inet::{ExtensionFlags, InetRequest, SocketId, StateFlags},
    SockDiagMessage,
};
use netlink_sys::{protocols::NETLINK_SOCK_DIAG, Socket, SocketAddr};

use super::FindProc;

pub struct FindProcImpl;

impl FindProc for FindProcImpl {
    fn resolve(
        src: Option<std::net::SocketAddr>,
        dst: Option<std::net::SocketAddr>,
        proto: i32,
    ) -> Option<String> {
        resolve(src, dst, proto)
    }
}

fn resolve(
    src: Option<std::net::SocketAddr>,
    dst: Option<std::net::SocketAddr>,
    proto: i32,
) -> Option<String> {
    if !crate::utils::check(src, dst) {
        return None;
    }
    if proto != libc::IPPROTO_TCP || proto != libc::IPPROTO_UDP {
        return None;
    }

    let (uid, inode) = resolve_uid_inode(src, dst, proto)?;
    resolve_process_name_by_proc_search(uid, inode).ok()
}

fn resolve_uid_inode(
    src: Option<std::net::SocketAddr>,
    dst: Option<std::net::SocketAddr>,
    proto: i32,
) -> Option<(u32, u32)> {
    let mut socket = Socket::new(NETLINK_SOCK_DIAG).unwrap();
    let _port_number = socket.bind_auto().unwrap().port_number();
    socket.connect(&SocketAddr::new(0, 0)).unwrap();

    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_DUMP;
    let (mut socket_id, family) = if crate::utils::is_ipv6(src, dst) {
        (SocketId::new_v6(), AF_INET6)
    } else {
        (SocketId::new_v4(), AF_INET)
    };
    if let Some(addr) = src {
        socket_id.source_address = addr.ip();
        socket_id.source_port = addr.port();
    }
    if let Some(addr) = dst {
        socket_id.destination_address = addr.ip();
        socket_id.destination_port = addr.port();
    }

    let mut packet = NetlinkMessage::new(
        nl_hdr,
        SockDiagMessage::InetRequest(InetRequest {
            family,
            protocol: proto as _,
            extensions: ExtensionFlags::empty(),
            states: StateFlags::all(),
            socket_id,
        })
        .into(),
    );

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];

    // Before calling serialize, it is important to check that the buffer in
    // which we're emitting is big enough for the packet, other
    // `serialize()` panics.
    assert_eq!(buf.len(), packet.buffer_len());

    packet.serialize(&mut buf[..]);

    if let Err(_) = socket.send(&buf[..], 0) {
        return None;
    }

    let mut receive_buffer = vec![0; 4096];
    let mut offset = 0;
    while let Ok(size) = socket.recv(&mut &mut receive_buffer[..], 0) {
        loop {
            let bytes = &receive_buffer[offset..];
            let rx_packet = <NetlinkMessage<SockDiagMessage>>::deserialize(bytes).unwrap();

            match rx_packet.payload {
                NetlinkPayload::Noop => {}
                NetlinkPayload::InnerMessage(SockDiagMessage::InetResponse(response)) => {
                    let uid = response.header.uid;
                    let inode = response.header.inode;
                    return Some((uid, inode));
                }
                _ => return None,
            }

            offset += rx_packet.header.length as usize;
            if offset == size || rx_packet.header.length == 0 {
                offset = 0;
                break;
            }
        }
    }
    None
}

fn resolve_process_name_by_proc_search(uid: u32, inode: u32) -> Result<String, io::Error> {
    let files = fs::read_dir("/proc")?;

    let mut buffer = [0; libc::PATH_MAX as usize];
    let socket = format!("socket:[{}]", inode).into_bytes();
    tracing::trace!("expected: {}", format!("socket:[{}]", inode));

    for file in files {
        let file = file?;
        let file_name = file.file_name();
        let file_name = file_name
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid file name"))?;

        if !file.metadata()?.is_dir() || !file_name.chars().all(char::is_numeric) {
            continue;
        }

        let uid_match = file.metadata()?.uid() == uid;
        if !uid_match {
            continue;
        }

        let process_path = format!("/proc/{}", file_name);
        let fd_path = format!("{}/fd", process_path);

        let fds = fs::read_dir(fd_path)?;
        for fd in fds {
            let fd = fd?;
            let fd_path = fd.path();

            let n = unsafe {
                libc::readlink(
                    fd_path.as_os_str().as_bytes().as_ptr() as *const libc::c_char,
                    buffer.as_mut_ptr() as *mut libc::c_char,
                    buffer.len(),
                )
            };
            if n == -1 {
                tracing::trace!("failed of {:?}", fd_path.to_str());
                continue;
            }

            let n = n as usize;
            let read_link = &buffer[..n];
            if read_link == &socket[..] {
                let cmdline = fs::read_to_string(Path::new(&process_path).join("cmdline"))?;
                tracing::trace!(
                    "process: {}, socket:{}, read_link:{}",
                    process_path,
                    String::from_utf8_lossy(&socket[..]),
                    String::from_utf8_lossy(read_link)
                );
                return Ok(split_cmdline(&cmdline));
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Process of uid({}), inode({}) not found", uid, inode),
    ))
}

fn split_cmdline(cmdline: &str) -> String {
    cmdline.split('\0').next().unwrap_or("").to_string()
}
