use anyhow::{Context, Result};
use maxminddb::{geoip2, Reader};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct GeoIpManager {
    reader: Arc<Reader<Vec<u8>>>,
    allowed_countries: Vec<String>,
    blocked_countries: Vec<String>,
}

impl GeoIpManager {
    pub fn new(
        database_path: &Path,
        allowed_countries: Vec<String>,
        blocked_countries: Vec<String>,
    ) -> Result<Self> {
        let reader = Reader::open_readfile(database_path)
            .context("Failed to open GeoIP database")?;

        debug!(
            "GeoIP database loaded: {} allowed countries, {} blocked countries",
            allowed_countries.len(),
            blocked_countries.len()
        );

        Ok(Self {
            reader: Arc::new(reader),
            allowed_countries,
            blocked_countries,
        })
    }

    pub fn is_allowed(&self, ip: IpAddr) -> Result<bool> {
        let country = self.lookup_country(ip)?;

        let country_code = match country {
            Some(code) => code,
            None => {
                debug!("No country found for IP {}, allowing by default", ip);
                return Ok(true);
            }
        };

        if !self.blocked_countries.is_empty() {
            if self.blocked_countries.contains(&country_code) {
                debug!("IP {} blocked (country: {})", ip, country_code);
                return Ok(false);
            }
        }

        if !self.allowed_countries.is_empty() {
            let allowed = self.allowed_countries.contains(&country_code);
            if !allowed {
                debug!("IP {} not in allowed countries (country: {})", ip, country_code);
            }
            return Ok(allowed);
        }

        Ok(true)
    }

    pub fn lookup_country(&self, ip: IpAddr) -> Result<Option<String>> {
        match self.reader.lookup::<geoip2::Country>(ip) {
            Ok(country) => {
                if let Some(c) = country.country {
                    if let Some(iso_code) = c.iso_code {
                        return Ok(Some(iso_code.to_string()));
                    }
                }
                Ok(None)
            }
            Err(e) => {
                warn!("GeoIP lookup failed for {}: {}", ip, e);
                Ok(None)
            }
        }
    }

    pub fn lookup_location(&self, ip: IpAddr) -> Result<Option<LocationInfo>> {
        match self.reader.lookup::<geoip2::City>(ip) {
            Ok(city) => {
                let country = city.country.and_then(|c| c.iso_code).map(|s| s.to_string());
                let city_name = city.city
                    .and_then(|c| c.names)
                    .and_then(|n| n.get("en").map(|s| s.to_string()));
                let continent = city.continent.and_then(|c| c.code).map(|s| s.to_string());

                if country.is_some() || city_name.is_some() {
                    Ok(Some(LocationInfo {
                        country,
                        city: city_name,
                        continent,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("GeoIP city lookup failed for {}: {}", ip, e);
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub country: Option<String>,
    pub city: Option<String>,
    pub continent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geoip_manager_requires_database() {

    }
}
