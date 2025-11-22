use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub profile_name: String,
    pub profile_class: ProfileClass,
    pub iccid: String,
    pub state: ProfileState,
    pub service_provider_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileClass {
    Test,
    Provisioning,
    Operational,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileState {
    Disabled,
    Enabled,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub iccid: String,
    pub isdp_aid: String,
    pub profile_state: ProfileState,
    pub profile_nickname: Option<String>,
    pub service_provider_name: String,
    pub profile_name: String,
    pub profile_class: ProfileClass,
}

impl ProfileInfo {
    pub fn new(iccid: String, service_provider_name: String, profile_name: String) -> Self {
        Self {
            iccid,
            isdp_aid: String::new(),
            profile_state: ProfileState::Disabled,
            profile_nickname: None,
            service_provider_name,
            profile_name,
            profile_class: ProfileClass::Operational,
        }
    }

    pub fn enable(&mut self) {
        self.profile_state = ProfileState::Enabled;
    }

    pub fn disable(&mut self) {
        self.profile_state = ProfileState::Disabled;
    }

    pub fn delete(&mut self) {
        self.profile_state = ProfileState::Deleted;
    }
}
