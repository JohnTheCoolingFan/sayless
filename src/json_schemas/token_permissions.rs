use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct TokenPermissions {
    #[serde(default)]
    pub admin_perm: bool,
    #[serde(default)]
    pub create_link_perm: bool,
    #[serde(default)]
    pub view_ips_perm: bool,
}

#[allow(dead_code)]
impl TokenPermissions {
    pub const fn new() -> Self {
        Self {
            admin_perm: false,
            create_link_perm: false,
            view_ips_perm: false,
        }
    }

    pub fn admin(mut self) -> Self {
        self.admin_perm = true;
        self
    }

    pub fn create_link(mut self) -> Self {
        self.create_link_perm = true;
        self
    }

    pub fn view_ips(mut self) -> Self {
        self.view_ips_perm = true;
        self
    }
}
