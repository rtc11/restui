use std::{
    collections::{BTreeMap, BTreeSet}, fmt::Display, fs::OpenOptions, hash::{DefaultHasher, Hash, Hasher}, io::Write, os::unix::fs::OpenOptionsExt
};

use futures::executor::block_on;
use k8s_openapi::api::{
    core::v1::{Pod, Secret},
    networking::v1::Ingress,
};
use serde::{Deserialize, Serialize};

use crate::{
    k8s::{self, Nais, NaisEnv, NaisIng},
    NAMESPACE,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    apps: BTreeMap<String, App>,
}

impl State {
    pub fn save(&self) {
        let path = std::env::current_dir()
            .expect("Failed to get current directory")
            .join("state.json");

        let file = OpenOptions::new()
            .mode(0o777)
            .write(true)
            .truncate(true)
            .open(&path);

        let mut file = match file {
            Ok(file) => file,
            Err(_) => OpenOptions::new()
                .append(true)
                .create(true)
                .open(&path)
                .expect("Failed to edit file"),
        };

        let state = serde_json::to_string(&self).unwrap();
        file.write_all(state.as_bytes())
            .expect("Failed to write to file");
    }

    pub fn insert(&mut self, app: App) {
        self.apps.insert(app.name.clone(), app);
    }

    pub fn remove(&mut self, app: &str) {
        self.apps.remove(app);
    }

    pub fn get(&self, app: &str) -> Option<&App> {
        self.apps.get(app)
    }

    pub fn get_mut(&mut self, app: &str) -> Option<&mut App> {
        self.apps.get_mut(app)
    }

    pub fn keys(&self) -> Vec<&str> {
        self.apps.keys().map(|k| k.as_str()).collect()
    }

    pub fn values(&self) -> Vec<&App> {
        self.apps.values().collect()
    }

    pub fn load() -> Self {
        match std::fs::read_to_string("state.json") {
            Ok(state) => serde_json::from_str(&state).expect(""),
            Err(_) => State::default(),
        }
    }

    async fn fetch_pods(&self) -> Vec<Pod> {
        k8s::pods(NAMESPACE)
            .await
            .expect("Failed to fetch k8s pods")
    }

    fn contains(&self, app: &str) -> bool {
        self.apps.contains_key(app)
    }

    pub fn update_apps(&mut self) {
        let pods = block_on(self.fetch_pods());

        for pod in pods {
            match self.contains(&pod.app_label()) {
                true => {
                    let app = self.get_mut(&pod.app_label()).unwrap();
                    app.pod = pod.metadata.name.clone().unwrap_or("???".into());
                    // self.insert(app);
                }
                false => {
                    let app = App::new(pod);
                    self.insert(app);
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub pod: String,
    pub cluster: String,
    pub ns: String,
    pub hosts: Vec<String>,
    pub azure: NaisOAuth2,
    pub requests: BTreeSet<Request>,
}

impl App {
    pub fn new(p: Pod) -> Self {
        let envs = p.app_envs();
        let name = p.app_label();
        let ns = p.metadata.namespace.clone().unwrap_or("???".into());

        Self {
            azure: NaisOAuth2::new(&name, &ns),
            pod: p.metadata.name.clone().unwrap_or("???".into()),
            cluster: envs.get_env("NAIS_CLUSTER_NAME").unwrap_or_default(),
            hosts: vec![],
            requests: BTreeSet::new(),
            name,
            ns,
        }
    }

    pub fn add_request(&mut self, request: Request) {
        self.requests.retain(|r| r.id != request.id);
        self.requests.insert(request);
    }

    pub fn update_hosts(&mut self) {
        let ingresses = block_on(self.fetch_ing());
        let hosts = ingresses.hosts();
        self.hosts = hosts;
    }

    async fn fetch_ing(&self) -> Vec<Ingress> {
        k8s::ings(&self.name, &self.ns)
            .await
            .expect("Failed to fetch k8s ingress")
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct NaisOAuth2 {
    name: String,
    ns: String,
    pub client_id: String,
    pub client_secret: String,
    pub token_endpoint: String,
}

impl NaisOAuth2 {
    pub fn new(app: &str, ns: &str) -> Self {
        Self {
            name: app.into(),
            ns: ns.into(),
            ..Default::default()
        }
    }

    async fn fetch_secret(&self) -> Secret {
        k8s::secret(&self.name, &self.ns)
            .await
            .expect("Failed to fetch k8s azure secret")
    }

    pub fn update(&mut self) {
        let secret = k8s::decode(block_on(self.fetch_secret()));
        self.client_id = secret["AZURE_APP_CLIENT_ID"].to_string();
        self.client_secret = secret["AZURE_APP_CLIENT_SECRET"].to_string();
        self.token_endpoint = secret["AZURE_OPENID_CONFIG_TOKEN_ENDPOINT"].to_string();
    }
}

#[derive(Default, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct Request {
    pub id: u64,
    pub method: Method,
    pub path: String,
    pub desc: String,
    pub headers: BTreeSet<Header>,
    pub body: String,
}

impl Request {
    pub fn new(method: Method, path: &str, headers: Vec<Header>, body: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        method.hash(&mut hasher);
        path.hash(&mut hasher);
        body.hash(&mut hasher);

        Self {
            id: hasher.finish(),
            method,
            path: path.into(),
            desc: "".into(),
            headers: headers.into_iter().collect(),
            body: body.into(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Clone, Debug, Hash)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Patch => write!(f, "PATCH"),
            Method::Delete => write!(f, "DELETE"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Clone, Debug, Default)]
pub struct Header {
    pub key: String,
    pub value: String,
}

impl Header {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}
