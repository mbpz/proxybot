import socket, struct, sys, binascii

HOST, PORT = "127.0.0.1", 1080

def recv_exact(s, n):
    buf = b""
    while len(buf) < n:
        chunk = s.recv(n - len(buf))
        if not chunk:
            raise RuntimeError("eof")
        buf += chunk
    return buf

def main():
    s = socket.create_connection((HOST, PORT), timeout=5)
    s.sendall(b"\x05\x01\x00")
    assert recv_exact(s, 2) == b"\x05\x00", "no-auth handshake failed"
    # UDP ASSOCIATE, address 0.0.0.0:0
    s.sendall(b"\x05\x03\x00\x01\x00\x00\x00\x00\x00\x00")
    head = recv_exact(s, 4)
    assert head[0] == 5 and head[1] == 0, head
    atyp = head[3]
    if atyp == 1:
        relay = socket.inet_ntoa(recv_exact(s, 4))
    elif atyp == 3:
        n = recv_exact(s, 1)[0]
        relay = recv_exact(s, n).decode()
    elif atyp == 4:
        relay = socket.inet_ntop(socket.AF_INET6, recv_exact(s, 16))
    else:
        raise RuntimeError("bad atyp")
    relay_port = struct.unpack("!H", recv_exact(s, 2))[0]
    print("UDP relay:", relay, relay_port)

    # DNS query for example.com A via 223.5.5.5:53
    q = b"\x12\x34\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00"
    for label in b"example.com".split(b"."):
        q += bytes([len(label)]) + label
    q += b"\x00\x00\x01\x00\x01"

    target_ip = "223.5.5.5"
    target_port = 53
    hdr = b"\x00\x00\x00\x01" + socket.inet_aton(target_ip) + struct.pack("!H", target_port)
    u = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    u.settimeout(5)
    u.sendto(hdr + q, (relay, relay_port))
    data, addr = u.recvfrom(2048)
    print("UDP response from", addr, "bytes", len(data))
    # strip SOCKS5 UDP response header: RSV(2) FRAG(1) ATYP(1) ADDR PORT
    i = 3
    atyp = data[i]; i += 1
    if atyp == 1:
        i += 4
    elif atyp == 3:
        i += 1 + data[i]
    elif atyp == 4:
        i += 16
    else:
        raise RuntimeError("bad response atyp")
    i += 2
    dns = data[i:]
    print("DNS response bytes:", len(dns), "txid:", dns[:2].hex(), "flags:", dns[2:4].hex(), "ancount:", struct.unpack("!H", dns[6:8])[0])
    print("PASS" if dns[:2] == b"\x12\x34" else "FAIL")

if __name__ == "__main__":
    main()
