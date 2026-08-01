//! macOS PF Adapter for transparent-proxy original-destination recovery.

use std::net::SocketAddr;

#[cfg(target_os = "macos")]
fn get_original_dst(peer_addr: SocketAddr, local_addr: SocketAddr) -> Option<SocketAddr> {
    use std::fs::OpenOptions;
    use std::net::IpAddr;
    use std::os::fd::AsRawFd;

    #[repr(C)]
    struct PfiocNatlook {
        saddr: [u8; 16],
        daddr: [u8; 16],
        rsaddr: [u8; 16],
        rdaddr: [u8; 16],
        sport: u16,
        dport: u16,
        rsport: u16,
        rdport: u16,
        af: u8,
        proto: u8,
        direction: u8,
        pad: [u8; 5],
    }

    const DIOCNATLOOK: libc::c_ulong = 0xC0544417;

    fn pack_ipv4(address: &IpAddr, output: &mut [u8; 16]) {
        output.fill(0);
        if let IpAddr::V4(address) = address {
            output[..4].copy_from_slice(&address.octets());
        }
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/pf")
        .ok()?;
    let mut lookup = PfiocNatlook {
        saddr: [0; 16],
        daddr: [0; 16],
        rsaddr: [0; 16],
        rdaddr: [0; 16],
        sport: peer_addr.port().to_be(),
        dport: local_addr.port().to_be(),
        rsport: 0,
        rdport: 0,
        af: 2,
        proto: 6,
        direction: 1,
        pad: [0; 5],
    };
    pack_ipv4(&peer_addr.ip(), &mut lookup.saddr);
    pack_ipv4(&local_addr.ip(), &mut lookup.daddr);

    // SAFETY: `lookup` has the C layout expected by DIOCNATLOOK and remains
    // alive and uniquely borrowed for the duration of ioctl.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            DIOCNATLOOK,
            &mut lookup as *mut _ as *mut libc::c_void,
        )
    };
    if result != 0 {
        return None;
    }
    let ip = std::net::Ipv4Addr::new(
        lookup.rdaddr[0],
        lookup.rdaddr[1],
        lookup.rdaddr[2],
        lookup.rdaddr[3],
    );
    Some(SocketAddr::new(IpAddr::V4(ip), u16::from_be(lookup.rdport)))
}

pub(super) fn get_original_dst_addr(socket: &tokio::net::TcpStream) -> Option<SocketAddr> {
    #[cfg(target_os = "macos")]
    {
        get_original_dst(socket.peer_addr().ok()?, socket.local_addr().ok()?)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = socket;
        None
    }
}
