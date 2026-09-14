use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpNetwork {
    V4 {
        network: u32,
        prefix_len: u8,
    },
    V6 {
        network: u128,
        prefix_len: u8,
    },
}

#[derive(Debug)]
pub struct NetworkList {
    networks: Vec<IpNetwork>,
}

impl NetworkList {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| format!("failed to read network list: {error}"))?;

        let mut networks = Vec::new();

        for (line_number, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            networks.push(
                parse_network(line).map_err(|error| {
                    format!("invalid network at line {}: {error}", line_number + 1)
                })?,
            );
        }

        Ok(Self { networks })
    }

    pub fn contains(&self, address: &IpAddr) -> bool {
        self.networks
            .iter()
            .any(|network| network.contains(address))
    }

    pub fn len(&self) -> usize {
        self.networks.len()
    }
}

impl IpNetwork {
    pub fn contains(&self, address: &IpAddr) -> bool {
        match (self, address) {
            (
                Self::V4 {
                    network,
                    prefix_len,
                },
                IpAddr::V4(address),
            ) => {
                let value = u32::from(*address);
                let mask = ipv4_mask(*prefix_len);

                value & mask == *network
            }

            (
                Self::V6 {
                    network,
                    prefix_len,
                },
                IpAddr::V6(address),
            ) => {
                let value = u128::from(*address);
                let mask = ipv6_mask(*prefix_len);

                value & mask == *network
            }

            _ => false,
        }
    }
}

fn parse_network(value: &str) -> Result<IpNetwork, String> {
    let (address_text, prefix_text) = value
        .split_once('/')
        .ok_or_else(|| format!("missing CIDR prefix in {value}"))?;

    let address = address_text
        .parse::<IpAddr>()
        .map_err(|error| format!("invalid IP address {address_text}: {error}"))?;

    let prefix_len = prefix_text
        .parse::<u8>()
        .map_err(|error| format!("invalid prefix length {prefix_text}: {error}"))?;

    match address {
        IpAddr::V4(address) => {
            if prefix_len > 32 {
                return Err(format!("IPv4 prefix length {prefix_len} exceeds 32"));
            }

            let mask = ipv4_mask(prefix_len);
            let network = u32::from(address) & mask;

            Ok(IpNetwork::V4 {
                network,
                prefix_len,
            })
        }

        IpAddr::V6(address) => {
            if prefix_len > 128 {
                return Err(format!("IPv6 prefix length {prefix_len} exceeds 128"));
            }

            let mask = ipv6_mask(prefix_len);
            let network = u128::from(address) & mask;

            Ok(IpNetwork::V6 {
                network,
                prefix_len,
            })
        }
    }
}

fn ipv4_mask(prefix_len: u8) -> u32 {
    if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len)
    }
}

fn ipv6_mask(prefix_len: u8) -> u128 {
    if prefix_len == 0 {
        0
    } else {
        u128::MAX << (128 - prefix_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ipv4_network() {
        let network = parse_network("104.16.0.0/13").expect("valid network");

        assert!(network.contains(&IpAddr::V4(Ipv4Addr::new(
            104, 21, 1, 21
        ))));

        assert!(!network.contains(&IpAddr::V4(Ipv4Addr::new(
            1, 1, 1, 1
        ))));
    }

    #[test]
    fn matches_ipv6_network() {
        let network =
            parse_network("2a06:98c0::/29").expect("valid network");

        let inside = "2a06:98c1:3120::3"
            .parse::<Ipv6Addr>()
            .expect("valid IPv6");

        let outside = "2606:4700::1"
            .parse::<Ipv6Addr>()
            .expect("valid IPv6");

        assert!(network.contains(&IpAddr::V6(inside)));
        assert!(!network.contains(&IpAddr::V6(outside)));
    }

    #[test]
    fn normalizes_non_network_address() {
        let network =
            parse_network("104.21.1.21/13").expect("valid network");

        assert_eq!(
            network,
            IpNetwork::V4 {
                network: u32::from(Ipv4Addr::new(104, 16, 0, 0)),
                prefix_len: 13,
            }
        );
    }

    #[test]
    fn rejects_invalid_prefix() {
        assert!(parse_network("104.16.0.0/33").is_err());
        assert!(parse_network("2606:4700::/129").is_err());
    }


    #[test]
    fn finds_unblocked_ipv4_neighbor() {
        use std::env;
        use std::fs;

        let network_path =
            env::temp_dir().join("laliga-dns-evade-network-test.txt");
        let blocklist_path =
            env::temp_dir().join("laliga-dns-evade-blocked-neighbor-test.txt");

        fs::write(&network_path, "104.16.0.0/13\n")
            .expect("network file should be written");

        fs::write(
            &blocklist_path,
            "104.21.1.21\n104.21.1.22\n",
        )
        .expect("blocklist should be written");

        let networks =
            NetworkList::load(&network_path).expect("networks should load");
        let blocked =
            crate::blocklist::Blocklist::load(&blocklist_path)
                .expect("blocklist should load");

        let address = IpAddr::V4(Ipv4Addr::new(104, 21, 1, 21));

        assert_eq!(
            find_evasive_address(&address, &networks, &blocked),
            Some(IpAddr::V4(Ipv4Addr::new(104, 21, 1, 20)))
        );

        let _ = fs::remove_file(network_path);
        let _ = fs::remove_file(blocklist_path);
    }

    #[test]
    fn does_not_leave_cloudflare_network() {
        use std::env;
        use std::fs;

        let network_path =
            env::temp_dir().join("laliga-dns-evade-network-boundary-test.txt");
        let blocklist_path =
            env::temp_dir().join("laliga-dns-evade-blocked-boundary-test.txt");

        fs::write(&network_path, "104.21.1.20/30\n")
            .expect("network file should be written");

        fs::write(
            &blocklist_path,
            "104.21.1.20\n104.21.1.21\n104.21.1.22\n104.21.1.23\n",
        )
        .expect("blocklist should be written");

        let networks =
            NetworkList::load(&network_path).expect("networks should load");
        let blocked =
            crate::blocklist::Blocklist::load(&blocklist_path)
                .expect("blocklist should load");

        let address = IpAddr::V4(Ipv4Addr::new(104, 21, 1, 21));

        assert_eq!(
            find_evasive_address(&address, &networks, &blocked),
            None
        );

        let _ = fs::remove_file(network_path);
        let _ = fs::remove_file(blocklist_path);
    }
}

pub fn find_evasive_address(
    address: &IpAddr,
    networks: &NetworkList,
    blocked: &crate::blocklist::Blocklist,
) -> Option<IpAddr> {
    let containing_network = networks
        .networks
        .iter()
        .find(|network| network.contains(address))?;

    match address {
        IpAddr::V4(address) => {
            let original = u32::from(*address);
            let base = original & 0xffff_ff00;
            let last = (original & 0xff) as i32;

            // Prefer a nearby address inside the same /24.
            for offset in 1..255_i32 {
                for candidate_last in [last + offset, last - offset] {
                    if !(1..=254).contains(&candidate_last) {
                        continue;
                    }

                    let candidate = base | candidate_last as u32;
                    let candidate_ip = IpAddr::V4(Ipv4Addr::from(candidate));

                    if containing_network.contains(&candidate_ip)
                        && !blocked.contains(&candidate_ip)
                    {
                        return Some(candidate_ip);
                    }
                }
            }

            // Fallback: search nearby addresses inside the same Cloudflare prefix.
            for offset in 1..65_536_u32 {
                for candidate in [
                    original.checked_add(offset),
                    original.checked_sub(offset),
                ]
                .into_iter()
                .flatten()
                {
                    let last_octet = candidate & 0xff;

                    if !(1..=254).contains(&last_octet) {
                        continue;
                    }

                    let candidate_ip = IpAddr::V4(Ipv4Addr::from(candidate));

                    if containing_network.contains(&candidate_ip)
                        && !blocked.contains(&candidate_ip)
                    {
                        return Some(candidate_ip);
                    }
                }
            }

            None
        }

        IpAddr::V6(address) => {
            let original = u128::from(*address);

            for offset in 1..1024_u128 {
                for candidate in [
                    original.checked_add(offset),
                    original.checked_sub(offset),
                ]
                .into_iter()
                .flatten()
                {
                    let candidate_ip = IpAddr::V6(Ipv6Addr::from(candidate));

                    if containing_network.contains(&candidate_ip)
                        && !blocked.contains(&candidate_ip)
                    {
                        return Some(candidate_ip);
                    }
                }
            }

            None
        }
    }
}
