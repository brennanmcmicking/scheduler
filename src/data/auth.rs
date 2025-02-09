use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Result};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
pub enum Authority {
    DISCORD,
    GOOGLE,
}

pub struct GoogleCsrfCookie {
    pub value: String,
}

#[derive(Clone, Serialize)]
struct DiscordTokenRequestBody {
    grant_type: String,
    code: String,
    redirect_uri: String,
}

#[derive(Clone, Deserialize)]
struct DiscordTokenResponseBody {
    access_token: String,
}

#[derive(Clone, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
}

#[derive(Clone)]
pub struct DiscordClient {
    pub http_client: reqwest::Client,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
}

impl DiscordClient {
    pub async fn get_user(&self, code: &str) -> Result<DiscordUser> {
        let body = DiscordTokenRequestBody {
            grant_type: "authorization_code".to_owned(),
            code: code.to_string(),
            redirect_uri: self.redirect_uri.clone(),
        };
        debug!("{}", serde_json::to_string_pretty(&body)?);
        debug!("requesting data for code={}", body.code);
        let response = self
            .http_client
            .post("https://discord.com/api/oauth2/token")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&body)
            .send()
            .await?
            .text()
            .await?;
        debug!("response text={}", response);

        let response = serde_json::from_str::<DiscordTokenResponseBody>(&response)
            .map_err(|e| anyhow!("failed to deserialize token response body, error={}", e))?;

        debug!("got auth token={}", response.access_token);

        let user = self
            .http_client
            .get("https://discord.com/api/users/@me")
            .bearer_auth(response.access_token)
            .send()
            .await?
            .json::<DiscordUser>()
            .await?;

        Ok(user)
    }
}
