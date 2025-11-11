use anyhow::Result;
use maxminddb::{MaxMindDBError, Reader};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

/// GeoIP lookup service
pub struct GeoIpService {
    reader: Arc<Reader<Vec<u8>>>,
}

impl GeoIpService {
    /// Create a new GeoIP service from a MaxMind database file
    pub fn new(database_path: &Path) -> Result<Self> {
        let reader = Reader::open_readfile(database_path)?;
        Ok(Self {
            reader: Arc::new(reader),
        })
    }

    /// Look up country code for an IP address
    pub fn lookup_country(&self, ip: IpAddr) -> Result<Option<String>> {
        match self.reader.lookup::<maxminddb::geoip2::Country>(ip) {
            Ok(country_data) => {
                if let Some(country) = country_data.country {
                    if let Some(iso_code) = country.iso_code {
                        return Ok(Some(iso_code.to_string()));
                    }
                }
                Ok(None)
            }
            Err(MaxMindDBError::AddressNotFoundError(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if IP is allowed based on whitelist/blacklist
    pub fn is_allowed(
        &self,
        ip: IpAddr,
        allowed_countries: &[String],
        blocked_countries: &[String],
    ) -> Result<bool> {
        let country = self.lookup_country(ip)?;

        if let Some(country_code) = country {
            // If there's a whitelist (allowed_countries not empty), only allow those countries
            if !allowed_countries.is_empty() {
                return Ok(allowed_countries.contains(&country_code));
            }

            // Otherwise, check blacklist
            if !blocked_countries.is_empty() {
                return Ok(!blocked_countries.contains(&country_code));
            }
        }

        // If no country found or no filters configured, allow by default
        Ok(true)
    }
}

/// GeoIP filter for WAF
pub struct GeoIpFilter {
    service: Option<GeoIpService>,
    allowed_countries: Vec<String>,
    blocked_countries: Vec<String>,
}

impl GeoIpFilter {
    /// Create a new GeoIP filter
    pub fn new(
        database_path: Option<&Path>,
        allowed_countries: Vec<String>,
        blocked_countries: Vec<String>,
    ) -> Result<Self> {
        let service = if let Some(path) = database_path {
            Some(GeoIpService::new(path)?)
        } else {
            None
        };

        Ok(Self {
            service,
            allowed_countries,
            blocked_countries,
        })
    }

    /// Check if request from IP should be allowed
    pub fn check(&self, ip: IpAddr) -> Result<bool> {
        if let Some(ref service) = self.service {
            service.is_allowed(ip, &self.allowed_countries, &self.blocked_countries)
        } else {
            // If no GeoIP service configured, allow all
            Ok(true)
        }
    }

    /// Get country for IP (for logging/debugging)
    pub fn get_country(&self, ip: IpAddr) -> Result<Option<String>> {
        if let Some(ref service) = self.service {
            service.lookup_country(ip)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_geoip_filter_no_database() {
        let filter = GeoIpFilter::new(None, vec![], vec![]).unwrap();

        // Should allow all when no database is configured
        let ip = IpAddr::from_str("8.8.8.8").unwrap();
        assert!(filter.check(ip).unwrap());
    }

    #[test]
    fn test_geoip_filter_no_restrictions() {
        let filter = GeoIpFilter::new(None, vec![], vec![]).unwrap();

        let ip = IpAddr::from_str("8.8.8.8").unwrap();
        assert!(filter.check(ip).unwrap());
    }

    #[test]
    #[ignore] // Requires GeoIP database file
    fn test_geoip_lookup() {
        // This test requires a GeoIP database file
        // Download from: https://dev.maxmind.com/geoip/geolite2-free-geolocation-data
        let db_path = Path::new("GeoLite2-Country.mmdb");
        if !db_path.exists() {
            return;
        }

        let service = GeoIpService::new(db_path).unwrap();

        // Google DNS (should be US)
        let ip = IpAddr::from_str("8.8.8.8").unwrap();
        let country = service.lookup_country(ip).unwrap();
        assert_eq!(country, Some("US".to_string()));
    }
}
