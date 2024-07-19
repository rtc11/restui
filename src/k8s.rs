use std::collections::BTreeMap;

use color_eyre::Result;
use k8s_openapi::api::{
    core::v1::{EnvVar, Pod, Secret},
    networking::v1::Ingress,
};
use kube::{api::ListParams, Api, Client};

use crate::{
    arg::{Cli, SecretType},
    util::StringJoin,
};


pub async fn pod(cli: Cli) -> Result<Pod> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, &cli.ns);
    let params = ListParams::default().labels(&format!("app={}", cli.app_name));
    let pods = pods.list(&params).await?;
    Ok(pods.items.first().cloned().unwrap())
}

pub async fn pods(ns: &str) -> Result<Vec<Pod>> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let params = ListParams::default();
    let pods = pods.list(&params).await?;
    Ok(pods.items)
}

pub async fn ings(app: &str, ns: &str) -> Result<Vec<Ingress>> {
    let client = Client::try_default().await?;
    let ings: Api<Ingress> = Api::namespaced(client, ns);
    let params = ListParams::default().labels(&format!("app={app}"));
    let ings = ings.list(&params).await?;
    Ok(ings.items)
}

pub async fn secret(app: &str, ns: &str) -> Result<Secret> {
    let client = Client::try_default().await?;
    let secrets: Api<Secret> = Api::namespaced(client, ns);
    let app_label = format!("app={app}");
    let secret_label: String = Into::into(&SecretType::Azure);
    let label = app_label.join_string(secret_label, ',');
    let lp = ListParams::default().labels(&label);
    let secrets = secrets.list(&lp).await?;

    assert!(
        secrets.items.len() == 1,
        "Expected 1 secret, found {}",
        secrets.items.len()
    );

    Ok(secrets.items.first().cloned().unwrap())
}

pub async fn secrets(ns: &str) -> Result<Vec<Secret>> {
    let client = Client::try_default().await?;
    let secrets: Api<Secret> = Api::namespaced(client, ns);
    let lp = ListParams::default();
    let secrets = secrets.list(&lp).await?;
    Ok(secrets.items)
}

pub trait Nais {
    fn app_label(&self) -> String;
    fn app_envs(&self) -> Vec<EnvVar>;
}

pub trait NaisEnv {
    fn get_env(&self, name: &str) -> Option<String>;
}

pub trait NaisIng {
    fn hosts(&self) -> Vec<String>;
}

impl NaisEnv for Vec<EnvVar> {
    fn get_env(&self, name: &str) -> Option<String> {
        self.iter()
            .filter(|e| e.name == *name)
            .collect::<Vec<_>>()[0]
            .value
            .clone()
    }
}

impl NaisIng for Vec<Ingress> {
    fn hosts(&self) -> Vec<String> {
        self.iter()
            .flat_map(|ing| ing.spec.clone().unwrap().rules.unwrap())
            .map(|h| h.host.clone().unwrap())
            .collect()
    }
}

impl Nais for Pod {
    fn app_label(&self) -> String {
        self.metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get("app"))
            .cloned()
            .unwrap_or("???".into())
    }

    fn app_envs(&self) -> Vec<EnvVar> {
        self.spec
            .clone()
            .expect("clone spec")
            .containers
            .into_iter()
            .filter(|c| c.name == self.app_label())
            .collect::<Vec<_>>()[0]
            .env
            .clone()
            .unwrap_or_default()
    }
}

pub async fn print(secret: Secret) {
    decode(secret).into_iter().for_each(|(key, value)| {
        println!("{:<12}: {}", key, value);
    });
}

pub struct SecretDisplayFilter {
    pub secret_name: String,
    pub display_name: String,
}

impl SecretDisplayFilter {
    pub fn new(secret_name: &str, display_name: &str) -> Self {
        Self {
            secret_name: secret_name.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

pub fn decode(secret: Secret) -> BTreeMap<String, String> {
    let mut res = BTreeMap::new();
    if let Some(data) = secret.data.clone() {
        for (k, v) in data {
            if let Ok(b) = std::str::from_utf8(&v.0) {
                res.insert(k, b.to_string());
            }
        }
    }
    res
}
