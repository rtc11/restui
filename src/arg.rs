use std::{collections::BTreeMap, process::exit};

pub fn parse() -> Cli {
    let mut cli = Cli::default();

    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let args_len = args.len();

    match args_len {
        0 | 1 => {
            println!("Usage: helved <Commands>\n");
            println!("Commands:");
            println!("-namespace  -n    k8s namespace");
            println!("-ingress    -i    host/ingress");
            println!("-app        -a    application name");
            println!("-method     -m    HTTP request methode [GET | POST | PUT | PATCH | DELETE]");
            println!("-path       -p    path");
            println!("-header     -h    add a header to header-list: <key>=<value>");
            println!("-body       -b    request body");
            println!("-filter     -f    filter on k8s secrets");
            exit(0);
        }
        _ if args_len % 2 != 0 => panic!("Invalid number of arguments"),
        _ => {
            args.chunks(2)
                .map(|chunk| (chunk[0].to_string(), chunk[1].to_string()))
                .for_each(|(key, value)| match (key.as_str(), value.as_str()) {
                    ("-namespace" | "-ns" | "-n", value) => {
                        cli.ns = value.into();
                    }
                    ("-app" | "-a", value) => {
                        cli.app_name = value.into();
                        cli.host = format!("{}.intern.dev.nav.no", cli.app_name);
                    }
                    ("-method" | "-m", value) => {
                        cli.method = match value.to_uppercase().as_str() {
                            "GET" => ArgMethod::Get,
                            "POST" => ArgMethod::Post,
                            "PUT" => ArgMethod::Put,
                            "PATCH" => ArgMethod::Patch,
                            "DELETE" => ArgMethod::Delete,
                            _ => panic!("Invalid method {value}"),
                        };
                    }
                    ("-ingress" | "-i" | "-host" | "-url" | "-u", value) => {
                        cli.host = value.into();
                    }
                    ("-path" | "-p", value) => {
                        cli.path = value.into();
                    }
                    ("-header" | "-h", value) => {
                        let header = value.split('=').collect::<Vec<&str>>();
                        if header.len() != 2 {
                            panic!("Invalid header {value}");
                        }
                        cli.headers.insert(header[0].into(), header[1].into());
                    }
                    ("-body" | "-b", value) => {
                        cli.body = value.into();
                    }
                    ("-filter" | "-f", value) => {
                        cli.filters.push(value.into());
                    }
                    _ => panic!("Invalid argument {key} {value}"),
                });
        }
    }

    cli
}

pub struct Cli {
    pub ns: String,
    pub app_name: String,
    pub method: ArgMethod,
    pub host: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: String,
    pub filters: Vec<String>,
}

pub enum ArgMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Default for Cli {
    fn default() -> Self {
        let app_name = String::from("utsjekk");
        let host = format!("http://{}.intern.dev.nav.no", app_name);

        Self {
            ns: "helved".into(),
            app_name,
            method: ArgMethod::Get,
            host,
            path: "/actuator/live".into(),
            headers: BTreeMap::new(),
            body: "".into(),
            filters: vec![
                "AZURE_APP_CLIENT_ID".into(),
                "AZURE_APP_CLIENT_SECRET".into(),
            ],
        }
    }
}

pub enum SecretType {
    Azure,
    Aiven,
}

impl From<&SecretType> for String {
    fn from(value: &SecretType) -> Self {
        match value {
            SecretType::Azure => format!("type={}", "azurerator.nais.io"),
            SecretType::Aiven => format!("type={}", "aivenator.aiven.nais.io"),
        }
    }
}
