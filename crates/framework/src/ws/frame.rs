use std::io::Read;
use std::net::TcpStream;
use crate::ws::{Opcode, Message, IoResult};

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> IoResult<()> {
    let mut off = 0;
    while off < buf.len() {
        match stream.read(&mut buf[off..]) {
            Ok(0) => return Err("Connection closed".into()),
            Ok(n) => off += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

pub fn read_frame(stream: &mut TcpStream, _buf: &mut Vec<u8>, frame_buf: &mut [u8; 2]) -> IoResult<Option<Message>> {
    read_exact(stream, frame_buf)?;
    let opcode = Opcode::from_u8(frame_buf[0]).ok_or_else(|| format!("Unknown opcode: {}", frame_buf[0]))?;
    let masked = (frame_buf[1] & 0x80) != 0;
    let mut payload_len = (frame_buf[1] & 0x7f) as u64;

    if payload_len == 126 {
        let mut ext = [0u8; 2];
        read_exact(stream, &mut ext)?;
        payload_len = u16::from_be_bytes(ext) as u64;
    } else if payload_len == 127 {
        let mut ext = [0u8; 8];
        read_exact(stream, &mut ext)?;
        payload_len = u64::from_be_bytes(ext);
    }

    let mut mask_key = [0u8; 4];
    if masked { read_exact(stream, &mut mask_key)?; }

    let mut payload = vec![0u8; payload_len as usize];
    if payload_len > 0 { read_exact(stream, &mut payload)?; }

    if masked {
        for (i, b) in payload.iter_mut().enumerate() { *b ^= mask_key[i % 4]; }
    }

    match opcode {
        Opcode::Close => {
            let (code, reason) = decode_close_payload(&payload);
            Ok(Some(Message::Close(code, reason)))
        }
        Opcode::Ping => Ok(Some(Message::Ping(payload))),
        Opcode::Pong => Ok(Some(Message::Pong(payload))),
        Opcode::Text => {
            let s = String::from_utf8(payload).map_err(|e| format!("Invalid UTF-8: {}", e))?;
            Ok(Some(Message::Text(s)))
        }
        Opcode::Binary => Ok(Some(Message::Binary(payload))),
        Opcode::Continue => Ok(None),
    }
}

pub fn encode_frame(opcode: Opcode, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode.as_u8());
    let len = payload.len();
    if len < 126 { frame.push(len as u8); }
    else if len <= 0xFFFF { frame.push(126); frame.extend_from_slice(&(len as u16).to_be_bytes()); }
    else { frame.push(127); frame.extend_from_slice(&(len as u64).to_be_bytes()); }
    frame.extend_from_slice(payload);
    frame
}

pub fn message_to_frame(msg: &Message) -> (Opcode, Option<Vec<u8>>) {
    match msg {
        Message::Text(s) => (Opcode::Text, Some(s.as_bytes().to_vec())),
        Message::Binary(d) => (Opcode::Binary, Some(d.clone())),
        Message::Ping(d) => (Opcode::Ping, Some(d.clone())),
        Message::Pong(d) => (Opcode::Pong, Some(d.clone())),
        Message::Close(code, reason) => (Opcode::Close, Some(encode_close_payload(*code, reason.as_deref()))),
    }
}

pub fn encode_close_payload(code: Option<u16>, reason: Option<&str>) -> Vec<u8> {
    let mut payload = Vec::new();
    let c = code.unwrap_or(1000);
    payload.extend_from_slice(&c.to_be_bytes());
    if let Some(r) = reason { payload.extend_from_slice(r.as_bytes()); }
    payload
}

pub fn decode_close_payload(payload: &[u8]) -> (Option<u16>, Option<String>) {
    if payload.len() >= 2 {
        let code = u16::from_be_bytes([payload[0], payload[1]]);
        let reason = if payload.len() > 2 { Some(String::from_utf8_lossy(&payload[2..]).to_string()) } else { None };
        (Some(code), reason)
    } else { (None, None) }
}
