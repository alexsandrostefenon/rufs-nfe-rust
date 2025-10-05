#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use rufs_base_rust::{rufs_micro_service::{RufsMicroService, RufsParams}, openapi::RufsOpenAPI, client::DataViewWatch};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
}

static REDIS_CLIENT: std::sync::OnceLock<redis::Client> = std::sync::OnceLock::new();

#[cfg(feature = "kafka")]
static MESSAGE_BROKER_PRODUCER: std::sync::OnceLock<samsa::prelude::Producer> = std::sync::OnceLock::new();

/*
use serde::Serialize;

#[derive(Serialize)]
struct NFe {
    // Defina os campos de acordo com o layout da NF-e
    infNFe: InfNFe,
}

#[derive(Serialize)]
struct InfNFe {
    // Campos da NF-e
    ide: Ide,
    emit: Emit,
    // ... outros campos
}

#[derive(Serialize)]
struct Ide {
    // Campos do identificador
    cUF: String,
    natOp: String,
    // ... outros campos
}

#[derive(Serialize)]
struct Emit {
    // Campos do emissor
    CNPJ: String,
    xNome: String,
    // ... outros campos
}

use serde_xml_rs::to_string;

fn generate_nfe_xml(nfe: &NFe) -> Result<String, serde_xml_rs::Error> {
    to_string(&nfe)
}

use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use openssl::x509::X509;
use openssl::sign::Signer;

fn sign_nfe(xml: &str, pkcs12_path: &str, pkcs12_password: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Carregar o arquivo PKCS#12
    let pkcs12 = std::fs::read(pkcs12_path)?;
    let parsed = Pkcs12::from_der(&pkcs12)?.parse(pkcs12_password)?;

    // Extrair a chave privada e o certificado
    let pkey: PKey<openssl::pkey::Private> = parsed.pkey;
    let cert: X509 = parsed.cert;

    // Criar o assinador
    let mut signer = Signer::new(openssl::hash::MessageDigest::sha256(), &pkey)?;
    signer.update(xml.as_bytes())?;

    // Gerar a assinatura
    let signature = signer.sign_to_vec()?;
    let signature_base64 = base64::encode(&signature);

    // Adicionar a assinatura ao XML (isso depende do layout da NF-e)

    // Retornar o XML assinado
    Ok(xml.to_string()) // Atualize isso conforme necessário
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Crie um exemplo de NF-e
    let nfe = NFe {
        infNFe: InfNFe {
            ide: Ide {
                cUF: "35".to_string(),
                natOp: "Venda de Mercadoria".to_string(),
                // ... outros campos
            },
            emit: Emit {
                CNPJ: "12345678901234".to_string(),
                xNome: "Empresa Exemplo".to_string(),
                // ... outros campos
            },
            // ... outros campos
        },
    };

    // Serializar a NF-e para XML
    let nfe_xml = generate_nfe_xml(&nfe)?;

    // Assinar o XML da NF-e
    let nfe_signed_xml = sign_nfe(&nfe_xml, "path/to/your/pkcs12.pfx", "your_password")?;

    // Imprimir o XML assinado
    println!("{}", nfe_signed_xml);

    Ok(())
}
*/
#[cfg(not(target_arch = "wasm32"))]
async fn server(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use rufs_base_rust::rufs_micro_service::RufsMicroServiceAuthenticator;
    use serde_json::Value;
    use rufs_nfe_rust::RufsNfe;

    #[derive(Clone)]
    pub struct State {
        authenticator: RufsMicroServiceAuthenticator
    }

    lazy_static::lazy_static! {
        static ref RUFS_STATE : State = State {
            authenticator: RufsMicroServiceAuthenticator()
        };
    }

    lazy_static::lazy_static! {
        static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
    }

    {
        let host = std::env::var("REDIS_HOST").unwrap_or("127.0.0.1".to_owned());
        let client = redis::Client::open(format!("redis://{host}/")).map_err(|err| format!("Redis failt : {err}"))?;
        REDIS_CLIENT.set(client).map_err(|_x| "Broker failt")?;
    }

    #[cfg(feature = "kafka")]
    {
        let query = std::env::var("CONNECT_BOOTSTRAP_SERVERS").unwrap_or("127.0.0.1:9092".to_owned());
        let re = regex::Regex::new(r"(?P<host>[\w\.\-]{1,64}):(?P<port>\d{1,5})").unwrap();
        let cap = re.captures(&query).unwrap();
        let host = cap.name("host").unwrap().as_str().to_string();
        let port: u16 = cap.name("port").unwrap().as_str().parse().unwrap();
        let bootstrap_addrs = vec![samsa::prelude::BrokerAddress {host, port}];

        let producer_builder = match samsa::prelude::ProducerBuilder::<samsa::prelude::TcpConnection>::new(bootstrap_addrs, vec!["nfe".to_string()]).await {
            Ok(producer_builder) => producer_builder,
            Err(err) => {
                eprintln!("{}", err);
                todo!()
            },
        };

        MESSAGE_BROKER_PRODUCER.set(producer_builder.build().await).map_err(|_| "Broker failt")?;
    }

    let app_name = "rufs_nfe".to_string();

    let params = RufsParams {
        openapi_file_name: format!("data/openapi-{}.json", app_name),
        app_name,
        ..Default::default()
    };

    let fs_prefix = {
        let test_path = "rufs-nfe-rust/";
        let path = std::env::current_dir()?.join(test_path);

        if path.is_dir() {
            test_path
        } else {
            ""
        }
    };

    println!("[rufs_nfe.main] std::env::current_dir() : {:?}", std::env::current_dir()?);
    println!("[rufs_nfe.main] fs_prefix = {:?}", fs_prefix);
    let db_uri = RufsMicroService::build_db_uri(None, None, None, None, Some(&params.app_name), None);
    println!("[rufs_nfe.main] db_uri = {}", db_uri);
    let mut rufs = RufsMicroService::connect(&db_uri, &format!("{}sql", fs_prefix), params, &WATCHER).await?;

    {
        use openapiv3::OpenAPI;
        use rufs_base_rust::openapi::FillOpenAPIOptions;
        let openapi_extra_str: &str = r##"{
            "openapi": "3.0.3",
            "info": {
                "title": "rufs-base-es6 openapi genetator",
                "version": "1.0.2"
            },
            "paths": {},
            "components": {
                "schemas": {
                    "upload": {
                    }
                }
            }
        }"##;
        let mut options = FillOpenAPIOptions::default();
        let openapi_extra = serde_json::from_str::<OpenAPI>(openapi_extra_str)?;
        options.schemas = openapi_extra.components.ok_or("missing section components")?.schemas.clone();
        rufs.openapi.fill(&mut options)?;

        if let Some(field) = rufs.openapi.get_property_mut("requestRepair", "request") {
            field.schema_data.extensions.insert("x-title".to_string(), Value::String("Equipamento para conserto/revisão".to_string()));
        }

        if let Some(field) = rufs.openapi.get_property_mut("requestProduct", "request") {
            field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de produtos/componentes".to_string()));
        }

        if let Some(field) = rufs.openapi.get_property_mut("requestService", "request") {
            field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de serviços".to_string()));
        }

        if let Some(field) = rufs.openapi.get_property_mut("requestPayment", "request") {
            field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de pagamentos".to_string()));
        }

        if let Some(field) = rufs.openapi.get_property_mut("person", "cnpjCpf") {
            field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
        }

        if let Some(field) = rufs.openapi.get_property_mut("person", "name") {
            field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
        }

        if let Some(field) = rufs.openapi.get_property_mut("account", "person") {
            field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
        }

        if let Some(field) = rufs.openapi.get_property_mut("account", "description") {
            field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
        }

        rufs.store_open_api()?;
    }

    #[cfg(feature = "warp")]
    #[cfg(not(feature = "tide"))]
    {
        use std::convert::Infallible;
        use warp::filters::path::FullPath;
        use warp::{Filter};
        use warp::Reply;
        use warp::{filters::multipart};
        use warp::http::StatusCode;

        macro_rules! warp_try {
            ($expr:expr) => {
                match $expr {
                    Ok(val) => val,
                    Err(err) => {
                        let err_str = err.to_string();
                        let str_status = &err_str[0..5];

                        let mut message = if (err_str.len() >= 5) {
                            &err_str[5..]
                        } else {
                            &err_str
                        };

                        let status = match str_status {
                            "401" => StatusCode::UNAUTHORIZED,
                            _ => {
                                message = &err_str;
                                StatusCode::BAD_REQUEST
                            }
                        };

                        let response = Box::new(warp::reply::with_status(message.to_string(), status)) as Box<dyn Reply>;
                        return Ok(response);
                    }
                }
            };
        }

        async fn handle_upload(rufs: Arc<Mutex<RufsMicroService<'static>>>, method: warp::http::Method, path: FullPath, headers: warp::http::HeaderMap, query: String, form: warp::multipart::FormData) -> Result<impl warp::Reply, Infallible> {
            let path = path.as_str();
            let method = method.to_string().to_lowercase();
            let mut headers_out: HashMap<String, String> = HashMap::new();

            for (name, value) in &headers {
                let key = name.to_string().to_lowercase();
                let value = warp_try!(value.to_str());
                headers_out.insert(key, value.to_string());
            }

            let rms = rufs.lock().await.to_owned();
            let token_payload = warp_try!(rufs_base_rust::request_filter::check_authorization::<RufsMicroService>(&rms, &headers_out, &path, &method).await);

            use std::collections::HashMap;
            use futures::{StreamExt, TryStreamExt};
            let mut parts = form.into_stream();
            let mut messages = vec![];

            while let Some(Ok(part)) = parts.next().await {
                if part.name() == "file" {
                    let re = regex::Regex::new(r"\bid=(?P<id>\d{44})").unwrap();

                    let Some(cap) = re.captures(&query) else {
                        continue;
                    };

                    let Some(id) = cap.name("id") else {
                        continue;
                    };

                    let Some(customer) = token_payload.extra.get("customer") else {
                        continue;
                    };

                    let Some(customer) = customer.as_str() else {
                        continue;
                    };

                    let file_path = format!("data/{}-{}-{}.html", customer, token_payload.name, id.as_str());

                    {
                        let mut file = warp_try!(tokio::fs::File::create(&file_path).await);
                        let mut stream = part.stream();

                        while let Some(chunk) = stream.next().await {
                            use tokio::io::AsyncWriteExt;
                            let mut chunk = warp_try!(chunk);
                            warp_try!(file.write_all_buf(&mut chunk).await);
                        }
                    }

                    let authorization = warp_try!(headers.get(&"Authorization".to_lowercase()).ok_or("Missing header Authorization")).to_str().unwrap();

                    let authorization = if authorization.starts_with("Bearer ") && authorization.len() > 7 {
                        &authorization[7..]
                    } else {
                        authorization
                    };

                    let message = json!({"authorization": authorization, "file": file_path});
                    #[cfg(feature = "kafka")]
                    {
                        let message = samsa::prelude::ProduceMessage {
                            partition_id: 0,
                            topic: "nfe".to_string(),
                            key: None,
                            value: Some(samsa::prelude::bytes::Bytes::from(message.to_string())),
                            headers: vec![],
                        };

                        warp_try!(MESSAGE_BROKER_PRODUCER.get().ok_or("broken broker")).produce(message).await;
                    }
                    #[cfg(not(feature = "kafka"))]
                    {
                        let mut con = warp_try!(warp_try!(REDIS_CLIENT.get().ok_or("broken redis")).get_connection());
                        let id: String = redis::Commands::xadd(&mut con, "nfe", "*", &[("data", message.to_string())]).unwrap();
                        println!("[rufs_nfe.upload] Redis Stream added entry with ID: {}", id);
                    }

                    messages.push(format!(r#"pushed event : {file_path}"#));
                }
            }

            use serde_json::json;
            let ret = warp::reply::json(&json!({"messages": messages}));
            return Ok(Box::new(ret));
        }

        let rufs = Arc::new(Mutex::new(rufs));
        let rufs_routes = rufs_base_rust::rufs_micro_service::rufs_warp(&rufs, &RUFS_STATE.authenticator).await;
        let listener = format!("127.0.0.1:{}", args.port);
        println!("[rufs_nfe.main] Staring server at {}", listener);
        let dedicated = warp::path("nfe_dedicated").and(warp::get()).map(|| {"Hello from rufs-nfe!".to_string()});
        //let cors = warp::cors().allow_any_origin().allow_methods(vec!["GET", "PUT", "OPTIONS", "POST", "DELETE"]).allow_headers(vec!["access-control-allow-origin","content-type"]);
        let routes = dedicated
            /*.or(warp::options().map(|| {
                "teste".to_string()
            }).with(cors.clone()))*/
            .or(rufs_routes)
            .or(warp::path("pkg").and(warp::fs::dir(format!("{}pkg", fs_prefix))))
            .or(warp::path("webapp").and(warp::fs::dir(format!("{}webapp", fs_prefix))))
            .or(warp::path::end().and(warp::fs::file(format!("{}webapp/index.html", fs_prefix)))
            .or(warp::path("upload").and(rufs_base_rust::rufs_micro_service::rufs_warp_with_rufs(rufs.clone())).and(warp::method()).and(warp::path::full()).and(warp::header::headers_cloned()).and(warp::query::raw()).and(multipart::form().max_length(1_000_000)).and_then(handle_upload))
            //.with(cors)
            );
        warp::serve(routes).run(([0, 0, 0, 0], args.port)).await;
    }

    #[cfg(feature = "tide")]
    #[cfg(not(feature = "warp"))]
    {
        let mut app = Box::new(tide::with_state(rufs));
        rufs_base_rust::rufs_micro_service::rufs_tide(&mut app).await?;
        let listener = format!("127.0.0.1:{}", args.port);
        println!("Staring rufs-nfe server at {}", listener);
        app.listen(listener).await?;
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = server(&args).await {
        println!("[rufs_nfe.main] ...server exited with error : {}", err);
        std::process::exit(1);
    }
}

// wasm-pack build --target web --dev
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    //use async_std::prelude::FutureExt;
    use futures_lite::future::FutureExt;
    use rufs_nfe_rust::RufsNfe;
    use rufs_base_rust::client::DataViewWatch;
    use crate::{server,Args};

    #[tokio::test]
    async fn selelium() -> Result<(), Box<dyn std::error::Error>> {
        let listening = async {
            println!("server()...");
            let args = Args { port: 8080 };
            server(&args).await
        };

        lazy_static::lazy_static! {
            static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
        }

        let selelium = async {
            println!("selelium() - sleep 5...");
            tokio::time::sleep( std::time::Duration::from_secs( 5 ) ).await;
            rufs_base_rust::client::tests::selelium(&WATCHER, "tests.side", "http://localhost:8080").await
        };

        {
            let db_uri = rufs_base_rust::rufs_micro_service::RufsMicroService::build_db_uri(None, None, None, None, Some("rufs_nfe"), None);
            println!("3 - build_db_uri : {}", db_uri);
            let (pg_conn, connection) = tokio_postgres::connect(&db_uri, tokio_postgres::NoTls).await?;
            println!("4 - connect : {}", db_uri);

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });

            pg_conn.execute(&format!("DROP SCHEMA IF EXISTS rufs_customer_12345678901 CASCADE"), &[]).await?;
        }

        listening.race(selelium).await?;
        println!("...selelium.");
        println!("...app.listen().");
        Ok(())
    }
}
