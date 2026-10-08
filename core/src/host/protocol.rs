use super::process::EcuProcess;
use super::profile::{DiagnosticProfile, Frame, Profile, Signal};
use crate::message::LocalizedText;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub(super) fn value(signal: &Signal, salt: u32) -> u32 {
    let mask = if signal.length == 32 {
        u32::MAX
    } else {
        (1u32 << signal.length) - 1
    };
    (0xA53C_71E9u32.rotate_right((signal.id % 31) as u32) ^ salt) & mask
}
pub(super) fn expected_payload(frame: &Frame, profile: &Profile, salt: u32) -> Vec<u8> {
    let mut data = vec![0u8; frame.dlc as usize];
    for signal in profile.signals.iter().filter(|s| s.frame == frame.path) {
        let v = value(signal, salt);
        for bit in 0..signal.length {
            let position = signal.start as usize + bit as usize;
            data[position / 8] |= (((v >> bit) & 1) as u8) << (position % 8);
        }
    }
    data
}
pub(super) fn parse_frame(event: &str) -> Result<(u32, Vec<u8>), LocalizedText> {
    let mut parts = event.split_whitespace();
    if parts.next() != Some("X") {
        return Err(crate::product_message!("backend.host.not_frame", "event" => event));
    }
    let id = parts
        .next()
        .ok_or_else(|| crate::product_message!("backend.host.frame_id_missing"))?
        .parse()
        .map_err(|_| crate::product_message!("backend.host.frame_id_invalid"))?;
    let dlc: usize = parts
        .next()
        .ok_or_else(|| crate::product_message!("backend.host.frame_dlc_missing"))?
        .parse()
        .map_err(|_| crate::product_message!("backend.host.frame_dlc_invalid"))?;
    let data = parts
        .next()
        .ok_or_else(|| crate::product_message!("backend.host.frame_payload_missing"))?;
    if parts.next().is_some() || data.len() != dlc * 2 {
        return Err(crate::product_message!("backend.host.frame_payload_length"));
    }
    let mut bytes = Vec::new();
    for pair in data.as_bytes().as_chunks::<2>().0 {
        bytes.push(
            u8::from_str_radix(std::str::from_utf8(pair).map_err(|e| e.to_string())?, 16)
                .map_err(|_| crate::product_message!("backend.host.frame_hex_invalid"))?,
        );
    }
    Ok((id, bytes))
}
pub(super) fn parse_value(response: &str) -> Result<(u32, bool), LocalizedText> {
    let words: Vec<_> = response.split_whitespace().collect();
    if words.len() != 4 || words[0] != "V" {
        return Err(
            crate::product_message!("backend.host.signal_response_invalid", "response" => response),
        );
    }
    Ok((
        words[2]
            .parse()
            .map_err(|_| crate::product_message!("backend.host.signal_value_invalid"))?,
        words[3] == "1",
    ))
}
pub(super) fn signal_value(payload: &[u8], signal: &Signal) -> u32 {
    let mut result = 0;
    for bit in 0..signal.length {
        let pos = signal.start as usize + bit as usize;
        result |= (((payload[pos / 8] >> (pos % 8)) & 1) as u32) << bit;
    }
    result
}
pub(super) fn prepare(
    ecu: &mut EcuProcess,
    profile: &Profile,
    salt: u32,
) -> Result<(), LocalizedText> {
    let commands: Vec<_> = profile
        .signals
        .iter()
        .filter(|s| profile.frames.iter().any(|f| f.tx && f.path == s.frame))
        .map(|s| format!("S {} {}", s.id, value(s, salt)))
        .collect();
    let (unexpected, _) = ecu.query(&commands, profile.signals[0].id)?;
    if !unexpected.is_empty() {
        return Err(crate::product_message!(
            "backend.host.transmitted_before_tick"
        ));
    }
    Ok(())
}

pub(super) fn tick(
    ecu: &mut EcuProcess,
    profile: &Profile,
    time: u64,
) -> Result<Vec<String>, LocalizedText> {
    let (frames, _) = ecu.query(&[format!("T {time}")], profile.signals[0].id)?;
    Ok(frames)
}
pub(super) fn hex_payload(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut text, "{byte:02X}").expect("writing to a String");
    }
    text
}

pub(super) fn diagnostic_frames(
    lines: &[String],
    expected: &[Vec<u8>],
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
) -> Result<(), LocalizedText> {
    let mut matched = 0;
    for line in lines {
        let (id, payload) = parse_frame(line)?;
        if id == diagnostic.response_id {
            if expected.get(matched) != Some(&payload) {
                return Err(
                    crate::product_message!("backend.host.diagnostic_frame_mismatch", "line" => line, "expected" => format!("{:?}", expected.get(matched))),
                );
            }
            matched += 1;
        } else {
            let frame = profile
                .frames
                .iter()
                .find(|frame| frame.tx && frame.id == id && frame.dlc as usize == payload.len())
                .ok_or_else(
                    || crate::product_message!("backend.host.unconfigured_frame", "line" => line),
                )?;
            if payload != expected_payload(frame, profile, salt) {
                return Err(
                    crate::product_message!("backend.host.periodic_payload_mismatch", "line" => line),
                );
            }
        }
    }
    if matched != expected.len() {
        return Err(
            crate::product_message!("backend.host.diagnostic_count", "actual" => matched, "expected" => expected.len()),
        );
    }
    Ok(())
}

pub(super) fn diagnostic_request(
    ecu: &mut EcuProcess,
    fence: u16,
    command: String,
) -> Result<Vec<String>, LocalizedText> {
    ecu.query(&[command], fence).map(|(frames, _)| frames)
}

pub(super) fn diagnostic_error(
    ecu: &mut EcuProcess,
    command: String,
    expected: &str,
) -> Result<(), LocalizedText> {
    ecu.command(&command)?;
    let line = ecu.next_line()?;
    if line == format!("E {expected}") {
        Ok(())
    } else {
        Err(
            crate::product_message!("backend.host.diagnostic_status_mismatch", "line" => line, "expected" => expected),
        )
    }
}

fn security_key(seed: &[u8]) -> [u8; 16] {
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for i in 0..32 {
        inner[i] ^= 0x5a;
        outer[i] ^= 0x5a;
    }
    let mut hash = Sha256::new();
    hash.update(inner);
    hash.update(b"AUTOSAR-HOST-SECURITY-v1");
    hash.update(seed);
    let digest = hash.finalize();
    let mut hash = Sha256::new();
    hash.update(outer);
    hash.update(digest);
    let digest = hash.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest[..16]);
    key
}

pub(super) fn send_payload(
    ecu: &mut EcuProcess,
    fence: u16,
    request: u32,
    payload: &[u8],
) -> Result<Vec<String>, LocalizedText> {
    if payload.len() <= 7 {
        let mut frame = vec![payload.len() as u8];
        frame.extend_from_slice(payload);
        return diagnostic_request(
            ecu,
            fence,
            format!("R {request} {} {}", frame.len(), hex_payload(&frame)),
        );
    }
    let mut first = vec![
        0x10 | ((payload.len() >> 8) as u8 & 0x0f),
        payload.len() as u8,
    ];
    first.extend_from_slice(&payload[..6]);
    let flow = diagnostic_request(
        ecu,
        fence,
        format!("R {request} {} {}", first.len(), hex_payload(&first)),
    )?;
    if flow.len() != 1 || parse_frame(&flow[0])?.1 != [0x30, 0x00, 0x00] {
        return Err(crate::product_message!("backend.host.cts_missing"));
    }
    let mut result = Vec::new();
    for (index, chunk) in payload[6..].chunks(7).enumerate() {
        let mut frame = vec![0x20 | ((index as u8 + 1) & 0x0f)];
        frame.extend_from_slice(chunk);
        result = diagnostic_request(
            ecu,
            fence,
            format!("R {request} {} {}", frame.len(), hex_payload(&frame)),
        )?;
    }
    Ok(result)
}

pub(super) fn security_seed(
    ecu: &mut EcuProcess,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
) -> Result<[u8; 16], LocalizedText> {
    let fence = profile.signals[0].id;
    let first = send_payload(ecu, fence, diagnostic.request_id, &[0x27, 0x01])?;
    let responses: Vec<_> = first
        .iter()
        .filter_map(|line| parse_frame(line).ok())
        .filter(|(id, _)| *id == diagnostic.response_id)
        .collect();
    if responses.len() != 1 || responses[0].1.len() != 8 || responses[0].1[..2] != [0x10, 0x12] {
        return Err(
            crate::product_message!("backend.host.seed_first_frame", "frames" => format!("{first:?}")),
        );
    }
    let mut payload = responses[0].1[2..].to_vec();
    let rest = diagnostic_request(ecu, fence, format!("R {} 3 300000", diagnostic.request_id))?;
    let mut sequence = 1u8;
    for line in &rest {
        let (id, frame) = parse_frame(line)?;
        if id == diagnostic.response_id {
            if frame.first() != Some(&(0x20 | sequence)) {
                return Err(crate::product_message!("backend.host.seed_sequence"));
            }
            payload.extend_from_slice(&frame[1..]);
            sequence += 1;
        }
    }
    payload.truncate(18);
    if sequence != 3 || payload.len() != 18 || payload[..2] != [0x67, 0x01] {
        return Err(crate::product_message!("backend.host.seed_payload"));
    }
    diagnostic_frames(&first, &[responses[0].1.clone()], profile, diagnostic, salt)?;
    let mut seed = [0u8; 16];
    seed.copy_from_slice(&payload[2..]);
    Ok(seed)
}

pub(super) fn security_unlock(
    ecu: &mut EcuProcess,
    profile: &Profile,
    diagnostic: &DiagnosticProfile,
    salt: u32,
) -> Result<[u8; 16], LocalizedText> {
    let seed = security_seed(ecu, profile, diagnostic, salt)?;
    let key = security_key(&seed);
    let mut key_request = vec![0x27, 0x02];
    key_request.extend_from_slice(&key);
    let result = send_payload(
        ecu,
        profile.signals[0].id,
        diagnostic.request_id,
        &key_request,
    )?;
    diagnostic_frames(
        &result,
        &[vec![0x02, 0x67, 0x02]],
        profile,
        diagnostic,
        salt,
    )?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_refusal_preserves_message_identity_and_original_evidence() {
        let event = "用户输入 / ECU: E FRAME_DLC";
        let error = parse_frame(event).unwrap_err();
        let wire = serde_json::to_value(&error).unwrap();
        assert_eq!(wire["key"], "backend.host.not_frame");
        assert_eq!(wire["params"]["event"], event);
        assert_eq!(error.to_string(), format!("Not a frame: {event}"));
    }

    #[test]
    fn successful_protocol_payloads_remain_machine_data() {
        assert_eq!(
            parse_frame("X 801 2 A53C").unwrap(),
            (801, vec![0xA5, 0x3C])
        );
        assert_eq!(parse_value("V 1 23 1").unwrap(), (23, true));
        assert_eq!(hex_payload(&[0xA5, 0x3C]), "A53C");
    }
}
