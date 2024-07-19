use color_eyre::Result;

pub trait OAuth2Body {
    fn body(&self) -> String;
}

pub struct ClientCredentials {
    client_id: String,
    client_secret: String,
    scope: String,
}

impl ClientCredentials {
    pub fn new(client_id: String, client_secret: String, scope: String) -> Self {
        Self {
            client_id,
            client_secret,
            scope,
        }
    }
}

pub struct OnBehalfOf {
    client_id: String,
    client_secret: String,
    access_token: String,
}

impl OnBehalfOf {
    pub fn new(client_id: String, client_secret: String, access_token: String) -> Self {
        Self {
            client_id,
            client_secret,
            access_token,
        }
    }
}

impl OAuth2Body for ClientCredentials {
    fn body(&self) -> String {
        format!(
            "client_id={}&client_secret={}&scope={}&grant_type=client_credentials",
            self.client_id, self.client_secret, self.scope
        )
    }
}

impl OAuth2Body for OnBehalfOf {
    fn body(&self) -> String {
        format!(
            "client_id={}&client_secret={}&grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&requested_token_use=on_behalf_of&assertion={}",
            self.client_id, self.client_secret, self.access_token
        )
    }
}

#[derive(serde::Deserialize)]
pub struct Token {
    pub expires_in: u16,
    pub access_token: String,
}

pub struct OAuth2Client {
    url: String,
}

impl OAuth2Client {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub async fn get_token<T: OAuth2Body>(&self, body: T) -> Result<Token> {
        let client = reqwest::Client::new();

        let res = client
            .post(&self.url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.body())
            .send()
            .await?;

        let token = res.json::<Token>().await?;
        Ok(token)
    }
}
