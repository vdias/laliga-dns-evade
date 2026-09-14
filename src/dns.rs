use std::net::{Ipv4Addr, Ipv6Addr};

const DNS_HEADER_LEN: usize = 12;
const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;

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
            _ => {}
        }

        offset += rdlength;
    }

    Ok(records)
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
    replacement: std::net::IpAddr,
) -> Result<(), String> {
    match (record, replacement) {
        (
            AddressRecord::A { offset, .. },
            std::net::IpAddr::V4(address),
        ) => {
            ensure_available(packet, *offset, 4)?;
            packet[*offset..*offset + 4].copy_from_slice(&address.octets());
            Ok(())
        }

        (
            AddressRecord::Aaaa { offset, .. },
            std::net::IpAddr::V6(address),
        ) => {
            ensure_available(packet, *offset, 16)?;
            packet[*offset..*offset + 16].copy_from_slice(&address.octets());
            Ok(())
        }

        (AddressRecord::A { .. }, std::net::IpAddr::V6(_)) => {
            Err("cannot replace an A record with an IPv6 address".to_string())
        }

        (AddressRecord::Aaaa { .. }, std::net::IpAddr::V4(_)) => {
            Err("cannot replace an AAAA record with an IPv4 address".to_string())
        }
    }
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
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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
