use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
};

#[derive(PartialEq, Eq, PartialOrd)]
pub struct Version<'a>(pub &'a str);

#[derive(PartialEq, Eq, Ord, PartialOrd)]
struct VersionParts {
    year: i32,
    month: i32,
    day: i32,
    patch1: i32,
    patch2: i32,
}

impl VersionParts {
    fn new(version: &str) -> Self {
        let parts: Vec<&str> = version.split('.').collect();

        Self {
            year: parts[0].parse::<i32>().unwrap(),
            month: parts[1].parse::<i32>().unwrap(),
            day: parts[2].parse::<i32>().unwrap(),
            patch1: parts[3].parse::<i32>().unwrap(),
            patch2: parts[4].parse::<i32>().unwrap(),
        }
    }
}

impl Display for Version<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Ord for Version<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let our_version_parts = VersionParts::new(self.0);
        let their_version_parts = VersionParts::new(other.0);

        our_version_parts.cmp(&their_version_parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq() {
        assert!(Version("2025.02.27.0000.0000") == Version("2025.02.27.0000.0000"));
        assert!(Version("2025.01.20.0000.0000") != Version("2025.02.27.0000.0000"));
    }

    #[test]
    fn test_ordering() {
        // year
        assert!(Version("2025.02.27.0000.0000") > Version("2024.02.27.0000.0000"));

        // month
        assert!(Version("2025.03.27.0000.0000") > Version("2025.02.27.0000.0000"));

        // day
        assert!(Version("2025.02.28.0000.0000") > Version("2025.02.27.0000.0000"));

        // patch1
        assert!(Version("2025.02.27.1000.0000") > Version("2025.02.27.0000.0000"));

        // patch2
        assert!(Version("2025.02.27.0000.1000") > Version("2025.02.27.0000.0000"));
    }
}
