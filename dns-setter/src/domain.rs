//! Domain types representing DNS providers, operations, and app state.

/// Represents different DNS providers with their server configurations.
#[derive(Debug, Clone, PartialEq)]
pub enum DnsProvider {
    Electro { primary: String, secondary: String },
    Radar { primary: String, secondary: String },
    Shekan { primary: String, secondary: String },
    Bogzar { primary: String, secondary: String },
    Quad9 { primary: String, secondary: String },
    Custom { primary: String, secondary: String },
}

impl DnsProvider {
    /// Create Electro DNS provider.
    pub fn electro() -> Self {
        Self::Electro {
            primary: "78.157.42.100".to_string(),
            secondary: "78.157.42.101".to_string(),
        }
    }

    /// Create Radar DNS provider.
    pub fn radar() -> Self {
        Self::Radar {
            primary: "10.202.10.10".to_string(),
            secondary: "10.202.10.11".to_string(),
        }
    }

    /// Create Shekan DNS provider.
    pub fn shekan() -> Self {
        Self::Shekan {
            primary: "178.22.122.100".to_string(),
            secondary: "185.51.200.2".to_string(),
        }
    }

    /// Create Bogzar DNS provider.
    pub fn bogzar() -> Self {
        Self::Bogzar {
            primary: "185.55.226.26".to_string(),
            secondary: "185.55.225.25".to_string(),
        }
    }

    /// Create Quad9 DNS provider.
    pub fn quad9() -> Self {
        Self::Quad9 {
            primary: "9.9.9.9".to_string(),
            secondary: "149.112.112.112".to_string(),
        }
    }

    /// Create custom DNS provider.
    pub fn custom(primary: String, secondary: String) -> Self {
        Self::Custom { primary, secondary }
    }

    /// Get DNS servers as tuple.
    pub fn get_servers(&self) -> (String, String) {
        match self {
            DnsProvider::Electro { primary, secondary }
            | DnsProvider::Radar { primary, secondary }
            | DnsProvider::Shekan { primary, secondary }
            | DnsProvider::Bogzar { primary, secondary }
            | DnsProvider::Quad9 { primary, secondary }
            | DnsProvider::Custom { primary, secondary } => (primary.clone(), secondary.clone()),
        }
    }

    /// Get display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            DnsProvider::Electro { .. } => "Electro",
            DnsProvider::Radar { .. } => "Radar",
            DnsProvider::Shekan { .. } => "Shekan",
            DnsProvider::Bogzar { .. } => "Bogzar",
            DnsProvider::Quad9 { .. } => "Quad9",
            DnsProvider::Custom { .. } => "Custom",
        }
    }

    // Get description for UI.
    // pub fn description(&self) -> &'static str {
    //     match self {
    //         DnsProvider::Electro { .. } => "Fast gaming DNS",
    //         DnsProvider::Radar { .. } => "Fast gaming DNS",
    //         DnsProvider::Shekan { .. } => "Fast gaming DNS",
    //         DnsProvider::Bogzar { .. } => "Fast gaming DNS",
    //         DnsProvider::Quad9 { .. } => "Security-focused",
    //         DnsProvider::Custom { .. } => "User-defined servers",
    //     }
    // }
}

/// Represents different DNS operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DnsOperation {
    Set(DnsProvider),
    Clear,
    Test,
}

/// Represents the result of a DNS operation.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationResult {
    Success(String),
    Error(String),
    Warning(String),
}

/// Represents the current state of the application.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppState {
    #[default]
    Idle,
    Processing,
    Success(String),
    Error(String),
    Warning(String),
}

/// Represents DNS configuration states.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DnsState {
    Static(Vec<String>),
    Dhcp,
    #[default]
    None,
}

impl Default for DnsProvider {
    fn default() -> Self {
        DnsProvider::electro()
    }
}
