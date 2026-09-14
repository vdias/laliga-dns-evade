use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const DNS_HEADER_LEN: usize = 12;
const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;
const TYPE_SVCB: u16 = 64;
const TYPE_HTTPS: u16 = 65;

const SVC_PARAM_IPV4HINT: u16 = 4;
const SVC_PARAM_IPV6HINT: u16 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressRecord {
    A {
        offset: usize,
        address: Ipv4Addr,
    },
    Aaaa {
        offset: usize,
        address: Ipv6Addr,
    },
    Ipv4Hint {
        offset: usize,
        address: Ipv4Addr,
    },
    Ipv6Hint {
        offset: usize,
        address: Ipv6Addr,
    },
}

impl AddressRecord {
    pub fn address(&self) -> IpAddr {
        match self {
            Self::A { address, .. } | Self::Ipv4Hint { address, .. } => IpAddr::V4(*address),
            Self::Aaaa { address, .. } | Self::Ipv6Hint { address, .. } => IpAddr::V6(*address),
        }
    }
}

pub fn extract_address_records(packet: &[u8]) -> Result<Vec<AddressRecord>, String> {
    if packet.len() < DNS_HEADER_LEN {
        return Err("DNS packet is shorter than the 12-byte header".to_string());
    }

    let qdcount = read_u16(packet, 4)?;
    let ancount = read_u16(packet, 6)?;
    let nscount = read_u16(packet, 8)?;
    let arcount = read_u16(packet, 10)?;

    let mut offset = DNS_HEADER_LEN;

    for _ in 0..qdcount {
        offset = skip_name(packet, offset)?;
        ensure_available(packet, offset, 4)?;
        offset += 4;
    }

    let record_count = ancount
        .checked_add(nscount)
        .and_then(|value| value.checked_add(arcount))
        .ok_or_else(|| "DNS record count overflow".to_string())?;

    let mut records = Vec::new();

    for _ in 0..record_count {
        offset = skip_name(packet, offset)?;

        ensure_available(packet, offset, 10)?;

        let record_type = read_u16(packet, offset)?;
        let rdlength = read_u16(packet, offset + 8)? as usize;

        offset += 10;

        ensure_available(packet, offset, rdlength)?;

        match (record_type, rdlength) {
            (TYPE_A, 4) => {
                records.push(AddressRecord::A {
                    offset,
                    address: Ipv4Addr::new(
                        packet[offset],
                        packet[offset + 1],
                        packet[offset + 2],
                        packet[offset + 3],
                    ),
                });
            }
            (TYPE_AAAA, 16) => {
                let mut octets = [0_u8; 16];
                octets.copy_from_slice(&packet[offset..offset + 16]);

                records.push(AddressRecord::Aaaa {
                    offset,
                    address: Ipv6Addr::from(octets),
                });
            }
            (TYPE_SVCB, _) | (TYPE_HTTPS, _) => {
                extract_svcb_address_hints(packet, offset, rdlength, &mut records)?;
            }
            _ => {}
        }

        offset += rdlength;
    }

    Ok(records)
}

fn extract_svcb_address_hints(
    packet: &[u8],
    rdata_offset: usize,
    rdlength: usize,
    records: &mut Vec<AddressRecord>,
) -> Result<(), String> {
    let rdata_end = rdata_offset
        .checked_add(rdlength)
        .ok_or_else(|| "SVCB/HTTPS RDATA offset overflow".to_string())?;

    ensure_available(packet, rdata_offset, rdlength)?;

    // SVCB/HTTPS RDATA begins with:
    //   SvcPriority: 2 bytes
    //   TargetName: DNS name
    //   SvcParams: repeated key/length/value tuples
    if rdlength < 3 {
        return Err("SVCB/HTTPS RDATA is too short".to_string());
    }

    let target_offset = rdata_offset + 2;
    let mut param_offset = skip_name(packet, target_offset)?;

    if param_offset > rdata_end {
        return Err("SVCB/HTTPS TargetName exceeds RDATA boundary".to_string());
    }

    while param_offset < rdata_end {
        if rdata_end - param_offset < 4 {
            return Err("truncated SVCB/HTTPS SvcParam header".to_string());
        }

        let key = read_u16(packet, param_offset)?;
        let value_len = read_u16(packet, param_offset + 2)? as usize;
        let value_offset = param_offset + 4;
        let value_end = value_offset
            .checked_add(value_len)
            .ok_or_else(|| "SVCB/HTTPS SvcParam length overflow".to_string())?;

        if value_end > rdata_end {
            return Err("SVCB/HTTPS SvcParam exceeds RDATA boundary".to_string());
        }

        match key {
            SVC_PARAM_IPV4HINT => {
                if value_len == 0 || value_len % 4 != 0 {
                    return Err(format!(
                        "invalid ipv4hint length {value_len}; expected a non-zero multiple of 4"
                    ));
                }

                for address_offset in (value_offset..value_end).step_by(4) {
                    records.push(AddressRecord::Ipv4Hint {
                        offset: address_offset,
                        address: Ipv4Addr::new(
                            packet[address_offset],
                            packet[address_offset + 1],
                            packet[address_offset + 2],
                            packet[address_offset + 3],
                        ),
                    });
                }
            }
            SVC_PARAM_IPV6HINT => {
                if value_len == 0 || value_len % 16 != 0 {
                    return Err(format!(
                        "invalid ipv6hint length {value_len}; expected a non-zero multiple of 16"
                    ));
                }

                for address_offset in (value_offset..value_end).step_by(16) {
                    let mut octets = [0_u8; 16];
                    octets.copy_from_slice(&packet[address_offset..address_offset + 16]);

                    records.push(AddressRecord::Ipv6Hint {
                        offset: address_offset,
                        address: Ipv6Addr::from(octets),
                    });
                }
            }
            _ => {}
        }

        param_offset = value_end;
    }

    Ok(())
}

fn skip_name(packet: &[u8], mut offset: usize) -> Result<usize, String> {
    loop {
        ensure_available(packet, offset, 1)?;

        let length = packet[offset];

        if length == 0 {
            return Ok(offset + 1);
        }

        if length & 0b1100_0000 == 0b1100_0000 {
            ensure_available(packet, offset, 2)?;
            return Ok(offset + 2);
        }

        if length & 0b1100_0000 != 0 {
            return Err(format!(
                "unsupported DNS label encoding at offset {offset}"
            ));
        }

        offset += 1;

        let label_len = length as usize;

        if label_len > 63 {
            return Err(format!(
                "invalid DNS label length {label_len} at offset {}",
                offset - 1
            ));
        }

        ensure_available(packet, offset, label_len)?;
        offset += label_len;
    }
}

fn read_u16(packet: &[u8], offset: usize) -> Result<u16, String> {
    ensure_available(packet, offset, 2)?;

    Ok(u16::from_be_bytes([
        packet[offset],
        packet[offset + 1],
    ]))
}

fn ensure_available(packet: &[u8], offset: usize, length: usize) -> Result<(), String> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| "DNS packet offset overflow".to_string())?;

    if end > packet.len() {
        return Err(format!(
            "DNS packet truncated: need bytes {offset}..{end}, packet length is {}",
            packet.len()
        ));
    }

    Ok(())
}

pub fn rewrite_address_record(
    packet: &mut [u8],
    record: &AddressRecord,
    replacement: IpAddr,
) -> Result<(), String> {
    match (record, replacement) {
        (
            AddressRecord::A { offset, .. } | AddressRecord::Ipv4Hint { offset, .. },
            IpAddr::V4(address),
        ) => {
            ensure_available(packet, *offset, 4)?;
            packet[*offset..*offset + 4].copy_from_slice(&address.octets());
            Ok(())
        }
        (
            AddressRecord::Aaaa { offset, .. } | AddressRecord::Ipv6Hint { offset, .. },
            IpAddr::V6(address),
        ) => {
            ensure_available(packet, *offset, 16)?;
            packet[*offset..*offset + 16].copy_from_slice(&address.octets());
            Ok(())
        }
        (
            AddressRecord::A { .. } | AddressRecord::Ipv4Hint { .. },
            IpAddr::V6(_),
        ) => Err("cannot replace an IPv4 DNS address with an IPv6 address".to_string()),
        (
            AddressRecord::Aaaa { .. } | AddressRecord::Ipv6Hint { .. },
            IpAddr::V4(_),
        ) => Err("cannot replace an IPv6 DNS address with an IPv4 address".to_string()),
    }
}

pub fn clear_authenticated_data(packet: &mut [u8]) -> Result<(), String> {
    if packet.len() < 4 {
        return Err("DNS packet too short to contain flags".to_string());
    }

    // DNS header flags occupy bytes 2-3.
    // AD (Authenticated Data) is bit 5 of the low-order flags byte.
    packet[3] &= !0x20;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ipv4_record() {
        let packet = [
            0x12, 0x34, 0x81, 0x80,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00,
            0x00, 0x00,
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
            0x03, b'c', b'o', b'm',
            0x00,
            0x00, 0x01,
            0x00, 0x01,
            0xc0, 0x0c,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00, 0x00, 0x3c,
            0x00, 0x04,
            203, 0, 113, 10,
        ];

        let records = extract_address_records(&packet).expect("valid DNS packet");

        assert_eq!(
            records,
            vec![AddressRecord::A {
                offset: packet.len() - 4,
                address: Ipv4Addr::new(203, 0, 113, 10),
            }]
        );
    }

    #[test]
    fn rejects_truncated_packet() {
        let packet = [0_u8; 5];

        assert!(extract_address_records(&packet).is_err());
    }
}

#[cfg(test)]
mod ipv6_tests {
    use super::*;

    #[test]
    fn extracts_ipv6_record() {
        let packet = [
            0x12, 0x34, 0x81, 0x80,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00,
            0x00, 0x00,
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
            0x03, b'c', b'o', b'm',
            0x00,
            0x00, 0x1c,
            0x00, 0x01,
            0xc0, 0x0c,
            0x00, 0x1c,
            0x00, 0x01,
            0x00, 0x00, 0x00, 0x3c,
            0x00, 0x10,
            0x20, 0x01, 0x0d, 0xb8,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x0a,
        ];

        let records = extract_address_records(&packet).expect("valid DNS packet");

        assert_eq!(
            records,
            vec![AddressRecord::Aaaa {
                offset: packet.len() - 16,
                address: Ipv6Addr::new(
                    0x2001, 0x0db8, 0, 0, 0, 0, 0, 10
                ),
            }]
        );
    }
}

#[cfg(test)]
mod rewrite_tests {
    use super::*;

    #[test]
    fn rewrites_ipv4_rdata_in_place() {
        let mut packet = [
            0x12, 0x34, 0x81, 0x80,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00,
            0x00, 0x00,
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
            0x03, b'c', b'o', b'm',
            0x00,
            0x00, 0x01,
            0x00, 0x01,
            0xc0, 0x0c,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00, 0x00, 0x3c,
            0x00, 0x04,
            104, 21, 1, 21,
        ];

        let records = extract_address_records(&packet).expect("valid DNS packet");

        rewrite_address_record(
            &mut packet,
            &records[0],
            IpAddr::V4(Ipv4Addr::new(104, 21, 1, 20)),
        )
        .expect("rewrite should succeed");

        let records = extract_address_records(&packet).expect("rewritten packet should parse");

        assert_eq!(
            records,
            vec![AddressRecord::A {
                offset: packet.len() - 4,
                address: Ipv4Addr::new(104, 21, 1, 20),
            }]
        );
    }

    #[test]
    fn rejects_address_family_mismatch() {
        let mut packet = vec![0_u8; 32];

        let record = AddressRecord::A {
            offset: 28,
            address: Ipv4Addr::new(104, 21, 1, 21),
        };

        let result = rewrite_address_record(
            &mut packet,
            &record,
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        );

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod svcb_https_tests {
    use super::*;

    fn service_binding_packet(record_type: u16) -> Vec<u8> {
        let mut packet = vec![
            0x12, 0x34, 0x81, 0x80,
            0x00, 0x01,
            0x00, 0x01,
            0x00, 0x00,
            0x00, 0x00,
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
            0x03, b'c', b'o', b'm',
            0x00,
        ];

        packet.extend_from_slice(&record_type.to_be_bytes());
        packet.extend_from_slice(&1_u16.to_be_bytes());

        packet.extend_from_slice(&[0xc0, 0x0c]);
        packet.extend_from_slice(&record_type.to_be_bytes());
        packet.extend_from_slice(&1_u16.to_be_bytes());
        packet.extend_from_slice(&60_u32.to_be_bytes());

        let rdata = [
            0x00, 0x01,
            0x00,
            0x00, 0x04,
            0x00, 0x08,
            104, 16, 132, 229,
            104, 16, 133, 229,
            0x00, 0x06,
            0x00, 0x10,
            0x26, 0x06, 0x47, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x11, 0x11,
        ];

        packet.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        packet.extend_from_slice(&rdata);

        packet
    }

    #[test]
    fn extracts_https_ipv4_and_ipv6_hints() {
        let packet = service_binding_packet(TYPE_HTTPS);
        let records = extract_address_records(&packet).expect("valid HTTPS DNS packet");

        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0].address(),
            IpAddr::V4(Ipv4Addr::new(104, 16, 132, 229))
        );
        assert_eq!(
            records[1].address(),
            IpAddr::V4(Ipv4Addr::new(104, 16, 133, 229))
        );
        assert_eq!(
            records[2].address(),
            IpAddr::V6(Ipv6Addr::new(
                0x2606, 0x4700, 0, 0, 0, 0, 0, 0x1111
            ))
        );
    }

    #[test]
    fn extracts_svcb_ipv4_and_ipv6_hints() {
        let packet = service_binding_packet(TYPE_SVCB);
        let records = extract_address_records(&packet).expect("valid SVCB DNS packet");

        assert_eq!(records.len(), 3);
        assert!(matches!(records[0], AddressRecord::Ipv4Hint { .. }));
        assert!(matches!(records[2], AddressRecord::Ipv6Hint { .. }));
    }

    #[test]
    fn rewrites_https_ipv4hint_in_place() {
        let mut packet = service_binding_packet(TYPE_HTTPS);
        let records = extract_address_records(&packet).expect("valid HTTPS DNS packet");

        let record = records
            .iter()
            .find(|record| {
                record.address() == IpAddr::V4(Ipv4Addr::new(104, 16, 132, 229))
            })
            .expect("ipv4hint record");

        rewrite_address_record(
            &mut packet,
            record,
            IpAddr::V4(Ipv4Addr::new(104, 16, 132, 230)),
        )
        .expect("ipv4hint rewrite should succeed");

        let rewritten = extract_address_records(&packet).expect("rewritten packet should parse");

        assert!(rewritten.iter().any(|record| {
            record.address() == IpAddr::V4(Ipv4Addr::new(104, 16, 132, 230))
        }));
    }

    #[test]
    fn rewrites_https_ipv6hint_in_place() {
        let mut packet = service_binding_packet(TYPE_HTTPS);
        let records = extract_address_records(&packet).expect("valid HTTPS DNS packet");

        let original = IpAddr::V6(Ipv6Addr::new(
            0x2606, 0x4700, 0, 0, 0, 0, 0, 0x1111,
        ));
        let replacement = IpAddr::V6(Ipv6Addr::new(
            0x2606, 0x4700, 0, 0, 0, 0, 0, 0x1112,
        ));

        let record = records
            .iter()
            .find(|record| record.address() == original)
            .expect("ipv6hint record");

        rewrite_address_record(&mut packet, record, replacement)
            .expect("ipv6hint rewrite should succeed");

        let rewritten = extract_address_records(&packet).expect("rewritten packet should parse");

        assert!(rewritten.iter().any(|record| record.address() == replacement));
    }
}

#[cfg(test)]
mod dnssec_flag_tests {
    use super::*;

    #[test]
    fn clears_authenticated_data_flag() {
        let mut packet = [
            0x12, 0x34,
            0x81, 0xa0, // QR, RD, RA, AD
            0x00, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            0x00, 0x00,
        ];

        clear_authenticated_data(&mut packet)
            .expect("AD flag should be cleared");

        assert_eq!(packet[3] & 0x20, 0);
        assert_eq!(packet[2], 0x81);
        assert_eq!(packet[3] & 0x80, 0x80); // RA preserved
    }

    #[test]
    fn rejects_short_packet_when_clearing_ad() {
        let mut packet = [0_u8; 3];

        assert!(clear_authenticated_data(&mut packet).is_err());
    }
}
