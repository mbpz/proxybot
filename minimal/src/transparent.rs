//! Transparent interception adapter.
//!
//! `pf` redirects device :80/:443 traffic to this listener. The adapter peeks at
//! the first bytes, discovers the destination host from TLS SNI or the HTTP Host
//! header, then hands the connection to the explicit MITM proxy on localhost by
//! synthesizing a CONNECT / absolute-form request. This keeps the mature
//! hudsucker MITM engine as the single interception implementation.

use anyhow::{Context, Result};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_PEEK: usize = 16 * 1024;

pub async fn serve(listen: SocketAddr, proxy: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("bind transparent listener on {listen}"))?;
    tracing::info!(%listen, %proxy, "transparent adapter listening");
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, "transparent accept failed");
                continue;
            }
        };
        tokio::spawn(async move {
            if let Err(error) = handle(stream, proxy).await {
                tracing::debug!(%peer, %error, "transparent connection closed");
            }
        });
    }
}

async fn handle(mut client: TcpStream, proxy: SocketAddr) -> io::Result<()> {
    let mut head = [0_u8; MAX_PEEK];
    let mut peeked = 0;
    while peeked < 5 {
        let n = client.peek(&mut head[peeked..]).await?;
        if n == 0 {
            return Ok(());
        }
        peeked += n;
        if peeked >= 5 {
            break;
        }
    }

    if head[0] == 0x16 && head[1] == 0x03 {
        let record_len = u16::from_be_bytes([head[3], head[4]]) as usize + 5;
        if record_len > MAX_PEEK {
            return Ok(());
        }
        // peek did not consume; read exactly the ClientHello record.
        let mut hello = vec![0_u8; record_len];
        client.read_exact(&mut hello).await?;
        let Some(sni) = parse_sni(&hello) else {
            tracing::debug!("TLS connection without SNI; skipping");
            return Ok(());
        };
        return tls_via_proxy(client, proxy, &sni, &hello).await;
    }

    if looks_like_http(&head[..peeked.min(head.len())]) {
        return http_via_proxy(client, proxy).await;
    }

    Ok(())
}

async fn tls_via_proxy(
    mut client: TcpStream,
    proxy: SocketAddr,
    sni: &str,
    hello: &[u8],
) -> io::Result<()> {
    let mut upstream = TcpStream::connect(proxy).await?;
    let connect = format!(
        "CONNECT {sni}:443 HTTP/1.1\r\nHost: {sni}:443\r\nProxy-Connection: keep-alive\r\n\r\n"
    );
    upstream.write_all(connect.as_bytes()).await?;
    let mut response = Vec::new();
    if !read_until_headers(&mut upstream, &mut response).await? {
        return Ok(());
    }
    if !response.starts_with(b"HTTP/1.1 200") && !response.starts_with(b"HTTP/1.0 200") {
        tracing::debug!(sni, "proxy refused CONNECT");
        return Ok(());
    }
    upstream.write_all(hello).await?;
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

async fn http_via_proxy(mut client: TcpStream, proxy: SocketAddr) -> io::Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let n = client.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..n]);
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
        if buffer.len() > MAX_PEEK {
            return Ok(());
        }
    };

    let head = &buffer[..header_end];
    let rest = &buffer[header_end..];
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split("\r\n");
    let Some(request_line) = lines.next() else {
        return Ok(());
    };
    let mut parts = request_line.split_whitespace();
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next()) else {
        return Ok(());
    };
    let host = find_header(&text, "host");
    let Some(host) = host else {
        return Ok(());
    };
    let absolute = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_string()
    } else {
        format!("http://{host}{target}")
    };

    let mut rewritten = String::new();
    rewritten.push_str(&format!("{method} {absolute} {version}\r\n"));
    for line in text.split("\r\n").skip(1) {
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("connection:") || lower.starts_with("proxy-connection:") {
            continue;
        }
        rewritten.push_str(line);
        rewritten.push_str("\r\n");
    }
    rewritten.push_str("Connection: close\r\n\r\n");

    let mut upstream = TcpStream::connect(proxy).await?;
    upstream.write_all(rewritten.as_bytes()).await?;
    if !rest.is_empty() {
        upstream.write_all(rest).await?;
    }
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    Ok(())
}

async fn read_until_headers(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> io::Result<bool> {
    let mut chunk = [0_u8; 1024];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(false);
        }
        buffer.extend_from_slice(&chunk[..n]);
        if find_header_end(buffer).is_some() {
            return Ok(true);
        }
        if buffer.len() > MAX_PEEK {
            return Ok(false);
        }
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn looks_like_http(bytes: &[u8]) -> bool {
    const METHODS: [&[u8]; 7] = [b"GET ", b"POST ", b"PUT ", b"HEAD ", b"DELETE ", b"OPTIONS ", b"PATCH "];
    METHODS.iter().any(|method| bytes.starts_with(method))
}

fn find_header(text: &str, name: &str) -> Option<String> {
    let prefix = format!("{}:", name.to_ascii_lowercase());
    text.lines()
        .find(|line| line.to_ascii_lowercase().starts_with(&prefix))
        .map(|line| line[prefix.len()..].trim().to_string())
}

/// Minimal TLS ClientHello SNI parser (RFC 6066). Returns None for no SNI.
pub fn parse_sni(record: &[u8]) -> Option<String> {
    if record.len() < 5 + 4 {
        return None;
    }
    if record[0] != 0x16 || record[1] != 0x03 {
        return None;
    }
    let mut cursor = 5; // skip TLS record header
    if *record.get(cursor)? != 0x01 {
        return None; // ClientHello
    }
    cursor += 4; // handshake header
    cursor += 2 + 32; // version + random
    let session_len = *record.get(cursor)? as usize;
    cursor += 1 + session_len;
    let cipher_len = u16::from_be_bytes([*record.get(cursor)?, *record.get(cursor + 1)?]) as usize;
    cursor += 2 + cipher_len;
    let compression_len = *record.get(cursor)? as usize;
    cursor += 1 + compression_len;
    if cursor + 2 > record.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([record[cursor], record[cursor + 1]]) as usize;
    cursor += 2;
    let extensions_end = (cursor + extensions_len).min(record.len());
    while cursor + 4 <= extensions_end {
        let kind = u16::from_be_bytes([record[cursor], record[cursor + 1]]);
        let len = u16::from_be_bytes([record[cursor + 2], record[cursor + 3]]) as usize;
        cursor += 4;
        if kind == 0x0000 {
            // server_name extension
            let data = record.get(cursor..cursor + len)?;
            if data.len() < 5 {
                return None;
            }
            let name_len = u16::from_be_bytes([data[3], data[4]]) as usize;
            let name = data.get(5..5 + name_len)?;
            return String::from_utf8(name.to_vec()).ok();
        }
        cursor += len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_http_methods() {
        assert!(looks_like_http(b"GET / HTTP/1.1\r\n"));
        assert!(looks_like_http(b"POST /x HTTP/1.1\r\n"));
        assert!(!looks_like_http(b"\x16\x03\x01"));
    }

    #[test]
    fn finds_header_end() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n"), Some(27));
    }
}
