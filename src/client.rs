use crate::{oauth2::{self, OAuth2Client}, state::NaisOAuth2};

pub struct Client {
    client: reqwest::Client,
    host: String,
    token: String,
}

impl Client {
    pub fn new(host: &String, token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            host: host.to_string(),
            token,
        }
    }

    pub async fn get(&self, path: &str) -> color_eyre::Result<()> {
        let res = self
            .client
            .get(format!("{}/{}", self.host, path))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        println!("{:?}", res.status());
        println!("{:?}", res.text().await?);
        Ok(())
    }

    pub async fn post(&self, path: &str, body: &str) -> color_eyre::Result<()> {
        let res = self
            .client
            .post(format!("{}/{}", self.host, path))
            .body(body.to_string())
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        println!("{:?}", res.status());
        println!("{:?}", res.text().await?);
        Ok(())
    }
}

pub async fn token(app: &str, ns: &str, azure: NaisOAuth2) -> color_eyre::Result<String> {
    let client = OAuth2Client::new(azure.token_endpoint);

    let client_credential = oauth2::ClientCredentials::new(
        azure.client_id, 
        azure.client_secret,
        format!("api://dev-gcp.{}.{}/.default", ns, app),
    );

    let token = client.get_token(client_credential).await?;
    Ok(token.access_token)
}
