#[cfg(not(target_arch = "wasm32"))]
async fn server(reset: bool) -> Result<Box<tide::Server<rufs_base_rust::rufs_micro_service::RufsMicroService<'static>>>, Box<dyn std::error::Error>> {
    use std::path::Path;
    use rufs_base_rust::{rufs_micro_service::RufsMicroService, micro_service_server::MicroServiceServer, openapi::RufsOpenAPI, rufs_tide_new};
    use serde_json::Value;

    if reset {
        let path = "openapi-rufs_nfe-rust.json";

        if Path::new(path).exists() {
            std::fs::remove_file(path)?;
        }

        let (db_client, connection) = tokio_postgres::connect("postgres://development:123456@localhost:5432/rufs_nfe_development", tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        println!("DROP DATABASE IF EXISTS rufs_nfe...");
        db_client.execute("DROP DATABASE IF EXISTS rufs_nfe", &[]).await?;        
        println!("...DROP DATABASE IF EXISTS rufs_nfe.");
        println!("CREATE DATABASE rufs_nfe...");
        db_client.execute("CREATE DATABASE rufs_nfe", &[]).await?;        
        println!("...CREATE DATABASE rufs_nfe.");
    }

    let base_dir = if std::env::current_dir()?.to_string_lossy().ends_with("/rufs-nfe-rust") {
        "./"
    } else {
        "./rufs-nfe-rust"
    };

    let mut rufs = RufsMicroService{
        check_rufs_tables: true,
        migration_path: Path::new(base_dir).join("sql").to_string_lossy().to_string(),
        micro_service_server: MicroServiceServer{
        app_name: "rufs_nfe".to_string(), ..Default::default()
        }, 
        ..Default::default()
    };

    rufs.connect(&format!("postgres://development:123456@localhost:5432/{}", rufs.micro_service_server.app_name)).await?;

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("requestProduct", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de produtos/componentes".to_string()));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("requestPayment", "request") {
        field.schema_data.extensions.insert("x-title".to_string(), Value::String("Lista de pagamentos".to_string()));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("person", "cnpjCpf") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("person", "name") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("account", "person") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    if let Some(field) = rufs.micro_service_server.openapi.get_property_mut("account", "description") {
        field.schema_data.extensions.insert("x-shortDescription".to_string(), Value::Bool(true));
    }

    rufs.micro_service_server.store_open_api("")?;
    rufs_tide_new(rufs, base_dir).await
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    use std::process::exit;
    #[cfg(debug_assertions)]
    let reset = true;
    #[cfg(not(debug_assertions))]
    let reset = false;

    let app = match server(reset).await {
        Ok(app) => app,
        Err(err) => {
            println!("...server exited with error : {}", err);
            exit(1);
        },
    };

    let rufs = app.state();
    let listen = format!("127.0.0.1:{}", rufs.micro_service_server.port);
    println!("Staring rufs-nfe server at {}", listen);

    match app.listen(listen).await {
        Ok(_) => println!("...server exited."),
        Err(err) => {
            println!("...server exited with error : {}", err);
            exit(2);
        },
    }
}
        
// wasm-pack build --target web --dev
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use async_std::prelude::FutureExt;
    use rufs_nfe_rust::RufsNfe;
    use std::time::Duration;
    use rufs_crud_rust::{DataViewWatch};
    use crate::{server};

    #[tokio::test]
    async fn selelium() -> Result<(), Box<dyn std::error::Error>> {
        println!("server()...");
        let app = server(true).await.unwrap();
        println!("...server().");
        let rufs = app.state();
        let listen = format!("127.0.0.1:{}", rufs.micro_service_server.port);

        let listening = async {
            println!("app.listen({})...", listen);
            app.listen(listen).await.unwrap()
        };

        let selelium = async {
            std::thread::sleep( Duration::from_secs( 1 ) );
            println!("selelium...");
            rufs_crud_rust::tests::selelium(&RufsNfe{} as &dyn DataViewWatch, "/home/alexsandro/Downloads/webapp-rust.side", "http://localhost:8080").await.unwrap()
        };

        listening.race(selelium).await;
        println!("...selelium.");
        println!("...app.listen().");
        Ok(())
    }
}
