use std::collections::HashSet;
use std::fs;
use std::net::IpAddr;
use std::path::Path;

#[derive(Debug)]
pub struct Blocklist {
    addresses: HashSet<IpAddr>,
}

impl Blocklist {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| format!("failed to read blocklist: {error}"))?;

        let mut addresses = HashSet::new();

        for (line_number, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let address = line.parse::<IpAddr>().map_err(|error| {
                format!(
                    "invalid IP address at line {}: {} ({error})",
                    line_number + 1,
                    line
                )
            })?;

            addresses.insert(address);
        }

        Ok(Self { addresses })
    }

    pub fn contains(&self, address: &IpAddr) -> bool {
        self.addresses.contains(address)
    }

    pub fn len(&self) -> usize {
        self.addresses.len()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn loads_ipv4_and_ipv6_addresses() {
        let path = env::temp_dir().join("laliga-dns-evade-blocklist-test.txt");

        fs::write(
            &path,
            "\
104.21.1.21
2a06:98c1:3120::3

# comment
104.21.1.21
",
        )
        .expect("test blocklist should be written");

        let blocklist = Blocklist::load(&path).expect("blocklist should load");

        assert_eq!(blocklist.len(), 2);

        assert!(blocklist.contains(&IpAddr::V4(Ipv4Addr::new(
            104, 21, 1, 21
        ))));

        assert!(blocklist.contains(&IpAddr::V6(
            "2a06:98c1:3120::3"
                .parse::<Ipv6Addr>()
                .expect("valid IPv6 address")
        )));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_address() {
        let path = env::temp_dir().join("laliga-dns-evade-invalid-blocklist-test.txt");

        fs::write(&path, "104.21.1.21\nnot-an-ip\n")
            .expect("test blocklist should be written");

        let result = Blocklist::load(&path);

        assert!(result.is_err());

        let _ = fs::remove_file(path);
    }
}
