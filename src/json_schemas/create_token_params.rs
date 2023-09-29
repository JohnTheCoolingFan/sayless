use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateTokenParams {
    #[serde(default)]
    pub admin_perm: bool,
    #[serde(default)]
    pub create_link_perm: bool,
    #[serde(default)]
    pub create_token_perm: bool,
    #[serde(default)]
    pub view_ips_perm: bool,
}
