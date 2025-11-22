use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Carrier information and SM-DP+ server details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierInfo {
    pub name: String,
    pub country: String,
    pub sm_dp_address: String,
    pub supports_esim: bool,
    pub requires_confirmation: bool,
    pub api_endpoint: Option<String>,
}

/// Global carrier database
/// NOTE: SM-DP+ addresses are examples - use actual carrier endpoints in production
pub struct CarrierDatabase {
    carriers: HashMap<String, CarrierInfo>,
}

impl CarrierDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            carriers: HashMap::new(),
        };
        db.populate_carriers();
        db
    }

    fn populate_carriers(&mut self) {
        // === UNITED STATES ===
        self.add_carrier("verizon", CarrierInfo {
            name: "Verizon Wireless".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "sm-v4-004-a-gtm.pr.go-esim.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.verizon.com/esim".to_string()),
        });

        self.add_carrier("att", CarrierInfo {
            name: "AT&T".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "sm-dp-plus.att.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.att.com/esim".to_string()),
        });

        self.add_carrier("tmobile", CarrierInfo {
            name: "T-Mobile USA".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "prod.smpc.t-mobile.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.t-mobile.com/esim".to_string()),
        });

        self.add_carrier("sprint", CarrierInfo {
            name: "Sprint (Now T-Mobile)".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "prod.smpc.t-mobile.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.t-mobile.com/esim".to_string()),
        });

        self.add_carrier("cricket", CarrierInfo {
            name: "Cricket Wireless".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "sm-dp-plus.att.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("uscellular", CarrierInfo {
            name: "U.S. Cellular".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "esim.uscellular.com".to_string(),
            supports_esim: true,
            requires_confirmation: true,
            api_endpoint: None,
        });

        // === INTERNATIONAL ===

        // UK
        self.add_carrier("ee", CarrierInfo {
            name: "EE (Everything Everywhere)".to_string(),
            country: "United Kingdom".to_string(),
            sm_dp_address: "sm-dp-plus.ee.co.uk".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("vodafone_uk", CarrierInfo {
            name: "Vodafone UK".to_string(),
            country: "United Kingdom".to_string(),
            sm_dp_address: "sm-dp-plus.vodafone.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.vodafone.com/esim".to_string()),
        });

        self.add_carrier("o2_uk", CarrierInfo {
            name: "O2 UK".to_string(),
            country: "United Kingdom".to_string(),
            sm_dp_address: "sm-dp-plus.o2.co.uk".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // Germany
        self.add_carrier("telekom_de", CarrierInfo {
            name: "Deutsche Telekom".to_string(),
            country: "Germany".to_string(),
            sm_dp_address: "prod.smdp.rsp.goog".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("vodafone_de", CarrierInfo {
            name: "Vodafone Germany".to_string(),
            country: "Germany".to_string(),
            sm_dp_address: "sm-dp-plus.vodafone.de".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // Canada
        self.add_carrier("rogers", CarrierInfo {
            name: "Rogers Wireless".to_string(),
            country: "Canada".to_string(),
            sm_dp_address: "sm-dp-plus.rogers.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("bell", CarrierInfo {
            name: "Bell Canada".to_string(),
            country: "Canada".to_string(),
            sm_dp_address: "sm-dp-plus.bell.ca".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("telus", CarrierInfo {
            name: "TELUS".to_string(),
            country: "Canada".to_string(),
            sm_dp_address: "sm-dp-plus.telus.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // Australia
        self.add_carrier("telstra", CarrierInfo {
            name: "Telstra".to_string(),
            country: "Australia".to_string(),
            sm_dp_address: "sm-dp-plus.telstra.com.au".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("optus", CarrierInfo {
            name: "Optus".to_string(),
            country: "Australia".to_string(),
            sm_dp_address: "sm-dp-plus.optus.com.au".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // Japan
        self.add_carrier("ntt_docomo", CarrierInfo {
            name: "NTT DoCoMo".to_string(),
            country: "Japan".to_string(),
            sm_dp_address: "sm-dp-plus.nttdocomo.co.jp".to_string(),
            supports_esim: true,
            requires_confirmation: true,
            api_endpoint: None,
        });

        self.add_carrier("softbank", CarrierInfo {
            name: "SoftBank".to_string(),
            country: "Japan".to_string(),
            sm_dp_address: "sm-dp-plus.softbank.jp".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // China
        self.add_carrier("china_mobile", CarrierInfo {
            name: "China Mobile".to_string(),
            country: "China".to_string(),
            sm_dp_address: "sm-dp-plus.chinamobile.com".to_string(),
            supports_esim: true,
            requires_confirmation: true,
            api_endpoint: None,
        });

        self.add_carrier("china_unicom", CarrierInfo {
            name: "China Unicom".to_string(),
            country: "China".to_string(),
            sm_dp_address: "sm-dp-plus.chinaunicom.com".to_string(),
            supports_esim: true,
            requires_confirmation: true,
            api_endpoint: None,
        });

        // === MVNO / VIRTUAL CARRIERS ===

        self.add_carrier("google_fi", CarrierInfo {
            name: "Google Fi".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "prod.smdp.rsp.goog".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://fi.google.com/api/esim".to_string()),
        });

        self.add_carrier("mint_mobile", CarrierInfo {
            name: "Mint Mobile".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "prod.smpc.t-mobile.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("visible", CarrierInfo {
            name: "Visible".to_string(),
            country: "United States".to_string(),
            sm_dp_address: "sm-v4-004-a-gtm.pr.go-esim.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        // === TRAVEL / INTERNATIONAL ESIM ===

        self.add_carrier("airalo", CarrierInfo {
            name: "Airalo (Global eSIM)".to_string(),
            country: "Global".to_string(),
            sm_dp_address: "sm-dp-plus.airalo.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: Some("https://api.airalo.com/v1".to_string()),
        });

        self.add_carrier("truphone", CarrierInfo {
            name: "Truphone (Global)".to_string(),
            country: "Global".to_string(),
            sm_dp_address: "sm-dp-plus.truphone.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });

        self.add_carrier("gigsky", CarrierInfo {
            name: "GigSky (Global)".to_string(),
            country: "Global".to_string(),
            sm_dp_address: "sm-dp-plus.gigsky.com".to_string(),
            supports_esim: true,
            requires_confirmation: false,
            api_endpoint: None,
        });
    }

    fn add_carrier(&mut self, id: &str, info: CarrierInfo) {
        self.carriers.insert(id.to_string(), info);
    }

    pub fn get_carrier(&self, id: &str) -> Option<&CarrierInfo> {
        self.carriers.get(id)
    }

    pub fn list_carriers(&self) -> Vec<(&String, &CarrierInfo)> {
        self.carriers.iter().collect()
    }

    pub fn list_by_country(&self, country: &str) -> Vec<(&String, &CarrierInfo)> {
        self.carriers
            .iter()
            .filter(|(_, info)| info.country == country)
            .collect()
    }

    pub fn search_carriers(&self, query: &str) -> Vec<(&String, &CarrierInfo)> {
        let query_lower = query.to_lowercase();
        self.carriers
            .iter()
            .filter(|(id, info)| {
                id.to_lowercase().contains(&query_lower)
                    || info.name.to_lowercase().contains(&query_lower)
                    || info.country.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn get_sm_dp_address(&self, carrier_id: &str) -> Option<String> {
        self.get_carrier(carrier_id).map(|info| info.sm_dp_address.clone())
    }
}

impl Default for CarrierDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carrier_database() {
        let db = CarrierDatabase::new();

        // Test US carriers
        assert!(db.get_carrier("verizon").is_some());
        assert!(db.get_carrier("att").is_some());
        assert!(db.get_carrier("tmobile").is_some());

        // Test international
        assert!(db.get_carrier("vodafone_uk").is_some());
        assert!(db.get_carrier("telstra").is_some());

        // Test search
        let results = db.search_carriers("verizon");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_country_filter() {
        let db = CarrierDatabase::new();
        let us_carriers = db.list_by_country("United States");
        assert!(us_carriers.len() > 0);
    }
}
