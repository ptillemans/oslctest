use std::{net::SocketAddr, collections::HashMap, sync::Arc, io::BufReader};

use axum::{Router, routing::get, extract::{Query, State}, response::Html};
use anyhow::{Context, Result};
use config::Config;
use reqwest::Url;
use serde::Deserialize;
use rio_xml::{RdfXmlParser, RdfXmlError};
use rio_api::parser::TriplesParser;
use rio_api::model::{NamedNode, Term};

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
struct Conf {
    root_url: String,
    client_id: String,
}

struct AppState {
    authorization_url: Url,
    client_id: String
}


#[tokio::main]
async fn main() {
    let cfg: Conf = Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("OSLC_"))
        .build().unwrap()
        .try_deserialize().unwrap();

    
    println!("config {:?}", cfg);


    let authorization_url = get_authorization_url(&cfg).await.unwrap();
    let client_id = cfg.client_id;
    
    let shared_state = Arc::new(AppState { authorization_url, client_id });
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/openid/callback", get(login_handler))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn root_handler(State(state): State<Arc<AppState>>) ->  Html<String> {
    let authorization_url = &state.authorization_url; 
    let client_id = &state.client_id; 
    Html(format!("<h1>Hello, World!</h1>
         <p>
           <a href=\"{}?response_type=code&client_id={}&scope=email+profile&redirect_uri=http://localhost:8888/openid/callback\">Login</a>
         </p>
         ", authorization_url, client_id ))
}

#[axum::debug_handler]
async fn login_handler(Query(params): Query<HashMap<String, String>>, body: String) -> Html<String> {
    let title = "<h1>Login Hander</h1>";
    let query = format!("<p>query params: {:?}</p>", params);
    let body = format!("<div><pre><![CDATA[{}]]><pre></div>", body);

    Html(format!("{}\n{}\n{}", title, query, body))

}

async fn get_authorization_url(cfg: &Conf) -> Result<Url> {

    let service_provider_url = format!("{}/sp/", cfg.root_url);
    println!("service_provider_url {}", service_provider_url);
    let body = reqwest::get(service_provider_url)
        .await?
        .text()
        .await?;
    let authorization_uri_predicate = NamedNode { iri: "http://open-services.net/ns/core#authorizationURI" };
    let mut url: Option<Url> = None;
    RdfXmlParser::new(BufReader::new(body.as_bytes()), None).parse_all(&mut |t| {
        println!("triple {:?}", t);
        if t.predicate == authorization_uri_predicate {
            if let Term::NamedNode(value) = t.object {
                let authorization_url = Url::parse(value.iri).unwrap();
                println!("\nfound authorization_uri_predicate {}\n", authorization_url);
                url = Some(authorization_url);
            }
        }
        Ok(()) as Result<(), RdfXmlError>
    })?;
    url.context("authorizationURI not found")
    
}
