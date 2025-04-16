use std::{net::SocketAddr, collections::HashMap, sync::Arc, io::BufReader};

use axum::{Router, routing::get, extract::{Query, State}, response::Html};
use anyhow::{Context, Result, bail};
use config::Config;
use reqwest::Url;
use serde::Deserialize;
use rio_xml::{RdfXmlParser, RdfXmlError};
use rio_api::parser::TriplesParser;
use rio_api::model::{NamedNode, Term};
use tokio::sync::Mutex;

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
struct Conf {
    root_url: String,
    client_id: String,
}

struct AppState {
    root_url: String,
    authorization_url: Url,
    client_id: String,
    authorization_code: Option<String>,
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
    let root_url = cfg.root_url;
    let client_id = cfg.client_id;
    let authorization_code = None;
    
    let shared_state = Arc::new(Mutex::new(AppState { 
        root_url,
        authorization_url,
        client_id,
        authorization_code 
    }));
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
async fn root_handler(State(state): State<Arc<Mutex<AppState>>>) ->  Html<String> {
    let state = state.lock().await;
    let state: &AppState = &state;
    let authorization_url = state.authorization_url.clone(); 
    let client_id = &state.client_id;
    let code = &state.authorization_code;
    let root_url = &state.root_url;
    match code {
       None => Html(format!("<h1>OSLC Test</h1>
         <p>
           <a href=\"{}?response_type=code&client_id={}&scope=email+profile&redirect_uri=http://localhost:8888/openid/callback\">Login</a>
         </p>
         ", authorization_url, client_id )),
       Some(code) => { 
           let content = get_some_content(root_url, code).await;
           Html(format!("<h1>OSLC Test</h1><p>{:?}</p>", content)) 
       },
    }

}

#[axum::debug_handler]
async fn login_handler(State(state): State<Arc<Mutex<AppState>>>, Query(params): Query<HashMap<String, String>> ) -> Html<String> {
    let mut state = state.lock_owned().await;
    let title = "<h1>Login Hander</h1>";
    let query = format!("<p>query params: {:?}</p>", params);

    let mut user_authentication_token = "*no token*".to_string();
    if let Some(code) = params.get("code") {
        println!("code: {:?}", code);
        state.authorization_code = Some(code.to_string());
        let token = get_user_authentication_token(&state.root_url, code).await.unwrap_or("error getting code".to_string());
        user_authentication_token = token;
    }

    Html(format!("{}\n{}\n<p>Token = {}", title, query, user_authentication_token))
}

async fn get_authorization_url(cfg: &Conf) -> Result<Url> {
    let service_provider_url = format!("{}/sp/", cfg.root_url);
    println!("service_provider_url {}", service_provider_url);
    let result = reqwest::get(service_provider_url)
        .await?;

    match result.status() {
        reqwest::StatusCode::OK => {
            let body = result.text().await?;
            let authorization_uri_predicate = NamedNode { iri: "http://open-services.net/ns/core#authorizationURI" };
            let mut url: Option<Url> = None;
            RdfXmlParser::new(BufReader::new(body.as_bytes()), None).parse_all(&mut |t| {
                if t.predicate == authorization_uri_predicate {
                    if let Term::NamedNode(value) = t.object {
                        let authorization_url = Url::parse(value.iri).unwrap();
                        println!("\nfound authorization_uri_predicate {}\n", authorization_url);
                        url = Some(authorization_url);
                    }
                };
                Ok::<(), RdfXmlError>(())
            })?;
            println!("\nurl: {:?}\n", url);
            url.context("authorizationURI not found")
        },
        _ => {
            println!("Error occured: {:?}", result);
            bail!("error getting authentication url")
        }
    }
}


async fn get_user_authentication_token(root_url: &str, code: &str) -> Result<String> {
    let logon_url = format!("{}/login/", root_url);
    let client = reqwest::Client::new();
    let response = client.post(logon_url)
        .body(format!("sso=openid;code={};redirecturi=http://localhost:8888/openid/callback", code))
        .send()
        .await?;
    println!("response: {:?}", response);
    println!("status: {:?}", response.status());
    let body = response.text().await?;
    println!("body: {}", body);
    let authentication_token_predicate = NamedNode { iri: "http://www.sparxsystems.com.au/oslc_am#useridentifier" };
    let mut token: Option<String> = None;
    RdfXmlParser::new(BufReader::new(body.as_bytes()), None).parse_all(&mut |t| {
        println!("triple: {:?}", t);
        println!("pred: {:?}", t.predicate);
        println!("test: {:?}", authentication_token_predicate);
        
        if t.predicate == authentication_token_predicate {
            println!("\nauthentication token value {:?}\n", t.object);
            if let Term::Literal(value) = t.object {
                let user_authentication_token = value.to_string();
                println!("\nfound authentication token {}\n", user_authentication_token);
                token = Some(user_authentication_token);
            }
        };
        Ok::<(), RdfXmlError>(())
    })?;
    token.context("authorizationURI not found")
}


async fn get_some_content(root_url: &str, code: &str) -> String {
    let client = reqwest::Client::new();
    //let query = format!("{}/qc/?oslc.where=dcterms:type=\"Actor\"", root_url);
    let query = format!("{}/resource/pk_{{4B9CF56D-77D3-462f-9179-D13876E5AC63}}?useridentifier={}", root_url, code);
    let body = client.get(&query)
        .send()
        .await
        .unwrap()
        .text()
        .await;
    format!("foo : from {:?}\n{:?}", query, body)
}

