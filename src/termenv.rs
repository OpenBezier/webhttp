use dashmap::DashMap;
use reqwest::Client;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone)]
pub struct Termenv {
    pub server: String,                  // Httl Url Addr
    pub urlprefix: String,               // URL Prefix
    pub client: Client,                  // HTTP's request client
    pub config: DashMap<String, String>, // other usable config
    pub user: Option<Value>,             // login info after logging
}

static TERMENV: OnceLock<Termenv> = OnceLock::<Termenv>::new();

pub fn termenv_init(server: String, urlprefix: String, cachedir: &str) -> &'static Termenv {
    TERMENV.get_or_init(|| {
        let client = reqwest::Client::builder()
            .http1_only()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(5))
            // .tcp_keepalive(std::time::Duration::from_secs(120))
            .build()
            .unwrap();

        let config = DashMap::<String, String>::default();
        config.insert("server".into(), server.clone());

        let mut datapath = directories::UserDirs::new()
            .unwrap()
            .home_dir()
            .to_path_buf();
        datapath.push(cachedir);
        std::fs::create_dir_all(&datapath).unwrap();
        let mut cachefile = datapath.clone();
        cachefile.push("config.json");
        config.insert("datapath".into(), datapath.to_string_lossy().to_string());
        config.insert("cachefile".into(), cachefile.to_string_lossy().to_string());

        let user = match termenv_read_access_token(cachefile) {
            Ok(user) => Some(user),
            Err(_e) => None,
        };
        let client: Termenv = Termenv {
            client,
            server,
            urlprefix: urlprefix,
            config,
            user: user,
        };
        client
    })
}

pub fn termenv_get() -> &'static Termenv {
    TERMENV.get().unwrap()
}

pub fn termenv_check() {
    if termenv_get().user.is_none() {
        println!("user access token is invalid, please login firstly");
        std::process::exit(1);
    }
}

fn termenv_read_access_token(config_file: PathBuf) -> anyhow::Result<Value> {
    let config_str = std::fs::read_to_string(config_file)?;
    let config = serde_json::from_str(&config_str)?;
    Ok(config)
}
