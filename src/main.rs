#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
use rufs_base_rust::{rufs_micro_service::{RufsMicroService, RufsParams}, openapi::RufsOpenAPI, client::DataViewWatch};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Parser, Debug)]
struct Args {
    #[arg(long,default_value = "8080")]
    port: u16,
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "clipp")]
async fn import_clipp(rufs: &RufsMicroService<'_>) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use convert_case::Casing;
    use rsfbclient::{prelude::*, FbError};
    use rufs_base_rust::entity_manager::EntityManager;
    use serde_json::{json, Value};

	fn get_json(row: &rsfbclient::Row) -> Result<Value, Box<dyn std::error::Error>> {
		let mut obj = json!({});

		for column in &row.cols {
			let value : Value = match &column.value {
                rsfbclient::SqlType::Text(value) => json!(value),
                rsfbclient::SqlType::Integer(value) => json!(value),
                rsfbclient::SqlType::Floating(value) => json!(value),
                rsfbclient::SqlType::Timestamp(value) => json!(value),
                rsfbclient::SqlType::Binary(_value) => serde_json::Value::Null,
                rsfbclient::SqlType::Boolean(value) => json!(value),
                rsfbclient::SqlType::Null => serde_json::Value::Null,
            };

			obj[column.name.to_case(convert_case::Case::Camel)] = value;
		}

		Ok(obj)
	}

    let mut fb_conn = rsfbclient::builder_native().from_string("firebird://SYSDBA:masterkey@localhost:3050//var/lib/firebird/3.0/data/CLIPP-3.fdb?charset=WIN1252")?.connect()?;//WIN1252,ISO8859_1

    let map_person = {
        let sql = "
        select
        left(nome, 255) as name,
        left(observacao, 255) as additional_data,
        cast(id_pais as integer) as country,
        end_cep as zip,--replace(end_cep,'-','') as zip,
        cast(id_cidade as integer) as city,
        end_bairro as district,
        coalesce(end_tipo,'') || ' ' || coalesce(end_lograd,'') as address,
        end_numero as address_number,
        end_comple as complement,
        email_cont as email,
        fone_celul as phone,
        insc_estad as ie_rg,
        insc_munic as im,
        nome_fanta as fantasy,
        c.id_cliente as id_import,
        coalesce(f.cpf,j.cnpj,lpad(cast(c.id_cliente as varchar(18)),15,'00.000.000/0000') || '-00') as cnpj_cpf
        from tb_cliente c left outer join tb_cli_pf f on c.id_cliente = f.id_cliente left outer join tb_cli_pj j on c.id_cliente = j.id_cliente order by id_import
        ";

        let mut map = HashMap::new();
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;

            let _obj_out = match rufs.entity_manager.insert(&rufs.openapi, "person", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "person_pkey""#) {
                        map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj.get("cnpjCpf").ok_or("broken cnpj_cpf")?.as_str().ok_or("broken str")?.to_string());
                        continue;
                    }

                    return Err(err)?;
                },
            };

            map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj.get("cnpjCpf").ok_or("broken cnpj_cpf")?.as_str().ok_or("broken str")?.to_string());
        }

        map
    };

    let mut map_request = HashMap::new();

    {
        /*
  NF_NUMERO integer NOT NULL,
  NF_SERIE varchar(3) CHARACTER SET WIN1252 NOT NULL COLLATE WIN_PTBR,
  NF_MODELO varchar(2) CHARACTER SET WIN1252 NOT NULL COLLATE WIN_PTBR,
  DT_EMISSAO date NOT NULL,
  DT_SAIDA date,
  HR_SAIDA time,
  ESPECIE varchar(15) CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  TIPO_FRETE char(1) CHARACTER SET WIN1252 NOT NULL COLLATE WIN_PTBR,
  PES_LIQUID numeric(18,4),
  PES_BRUTO numeric(18,4),
  STATUS char(1) CHARACTER SET WIN1252 NOT NULL COLLATE WIN_PTBR,
  ENT_SAI char(1) CHARACTER SET WIN1252 NOT NULL COLLATE WIN_PTBR,
  ID_FMAPGTO smallint NOT NULL,
  ID_PARCELA smallint NOT NULL,
  MARCA varchar(15) CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  QTD_VOLUM numeric(18,4),
  NUM_VOLUM varchar(15) CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  PROD_REV char(1) CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  SOMA_FRETE char(1) CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  VLR_TROCO numeric(18,4) DEFAULT 0,
  IND_PRES char(1),
  IND_IE_DEST char(1),
  DESCONTO_CONDICIONAL char(1) CHARACTER SET WIN1252 DEFAULT 'S' NOT NULL COLLATE WIN_PTBR,
  INF_COMP_FIXA blob sub_type 1 CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  INF_COMP_EDIT blob sub_type 1 CHARACTER SET WIN1252 COLLATE WIN_PTBR,
  ENDERECO_ENTREGA char(1) DEFAULT 'N' NOT NULL,
  ENVIO_API timestamp,
          
	date_in_out timestamp default CURRENT_TIMESTAMP,--dhSaiEnt
	versao varchar(4) default '3.10',
	nfe_id char(47),
	--natOp varchar(60) default 'VENDA',
	--indPag integer default 0,-- 0=Pagamento à vista; 1=Pagamento a prazo; 2=Outros.
	mod integer default 55,-- 55=NF-e emitida em substituição ao modelo 1 ou 1A; 65=NFC-e, utilizada nas operações de venda no varejo (a critério da UF aceitar este modelo de documento).
	serie integer default 1,
	numero integer GENERATED BY DEFAULT AS IDENTITY not null,--nNF
	--(request.type-1) --tpNF integer default 1,-- 0=Entrada; 1=Saída
	--idDest integer default 1,-- 1=Operação interna; 2=Operação interestadual; 3=Operação com exterior.
	tp_imp integer default 1,-- 0=Sem geração de DANFE; 1=DANFE normal, Retrato; 2=DANFE normal, Paisagem; 3=DANFE Simplificado; 4=DANFE NFC-e; 5=DANFE NFC-e somente em mensagem eletrônica
	tp_emis integer default 1,
	c_dv integer,-- DV da Chave de Acesso da NF-e, o DV será calculado com a aplicação do algoritmo módulo 11 (base 2,9) da Chave de Acesso. (vide item 5 do Manual de Orientação)
	--tpAmb integer default 1,-- 1=Produção/2=Homologação
	--finNFe integer default 1,-- 1=NF-e normal; 2=NF-e complementar; 3=NF-e de ajuste; 4=Devolução de mercadoria.
	ind_final integer default 1, -- 0=Normal; 1=Consumidor final;
	ind_pres integer default 1, -- 0=Não se aplica (por exemplo, Nota Fiscal complementar ou de ajuste); 1=Operação presencial; 2=Operação não presencial, pela Internet; 3=Operação não presencial, Teleatendimento; 4=NFC-e em operação com entrega a domicílio; 9=Operação não presencial, outros
	proc_emi integer default 0,
	ver_proc varchar(20) default '1.0.000',
	ind_ie_dest integer default 9,--,-- 1=Contribuinte ICMS (informar a IE do destinatário); 2=Contribuinte isento ICMS; 9=Não Contribuinte, que pode ou não possuir Inscrição Estadual
        */
        let sql = r#"
        select
        2 as type,
        320 as state,
        '90.979.337/0001-85' as person,
        id_cliente,
        (dt_saida || 'T' || hr_saida) as "date_in_out",
        (dt_saida || 'T' || hr_saida) as "date",
        NF_NUMERO as numero,
        cast(NF_SERIE as int) as serie,
        cast(NF_MODELO as int) as mod,
        ENT_SAI,
        left(INF_COMP_EDIT,255) as additional_data,
        ID_NFVENDA as id_import
        from tb_nfvenda order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let client_id = obj.get("idCliente").ok_or("broken")?.as_u64().ok_or("broken")?;
            obj["personDest"] = json!(map_person.get(&client_id));

            let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "request_person_person_dest_date_key""#) {
                        continue;
                    }

                    return Err(err)?;
                },
            };

            let request_id = obj_out.get("id").ok_or("broken request id")?.as_u64().ok_or("broken u64")?;
            map_request.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, request_id);
            obj["request"] = json!(request_id);

            let _obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_nfe", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "request_person_person_dest_date_key""#) {
                        continue;
                    }

                    return Err(err)?;
                },
            };
        }
    }

    {
        let sql = r#"
        select
        id_status,
        2 as type,
        id_status as state,
        '90.979.337/0001-85' as person,
        id_cliente,
        (dt_os || 'T' || hr_os) as "date",
        left(observacao,255) as additional_data,
        id_os*10000 as id_import
        from tb_os order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let client_id = obj.get("idCliente").ok_or("broken")?.as_u64().ok_or("broken")?;
            obj["personDest"] = json!(map_person.get(&client_id));
            let obj_id_status_in = obj.get("idStatus").ok_or("Missing id_status")?.as_u64().ok_or("broken u64")?;

            for (id_in, id_out) in vec![(1, 220),(2, 250), (3, 270), (4, 280), (5, 250), (6, 220), (7, 260), (9, 320), (10, 310), (11, 240), (12, 320)] {
                if obj_id_status_in == id_in {
                    obj["state"] = json!(id_out);
                    break;
                }
            }

            let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    if err.to_string().contains(r#"duplicate key value violates unique constraint "request_person_person_dest_date_key""#) {
                        continue;
                    }

                    return Err(err)?;
                },
            };

            map_request.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out.get("id").ok_or("broken request id")?.as_u64().ok_or("broken u64")?);
        }
    }

    let (map_product, map_service) = {
        let sql = r#"
        select
        left(descricao, 120) as name,
        left(observacao, 255) as description,
        left(lower(uni_medida), 2) as unity,
        prc_venda as "value",
        prc_custo as value_cost,
        margem_lb as margin_sale,
        (case grade_serie when 'S' then true else false end) as serial_control,
        id_tipoitem,
        id_estoque as id_import
        from tb_estoque order by id_import
        "#;

        let mut map_product = HashMap::new();
        let mut map_service = HashMap::new();
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let obj = get_json(&row?)?;
            let id_tipo_item = obj.get("idTipoitem").ok_or("broken id_tipoitem")?.as_str().ok_or("broken id_tipoitem str")?;
            let id_import = obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?;

            if ["9"].contains(&id_tipo_item) {
                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "service", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };

                map_service.insert(id_import, obj_out.get("id").ok_or("broken service id")?.as_u64().ok_or("broken service id u64")?);
            } else {
                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "product", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };

                map_product.insert(id_import, obj_out.get("id").ok_or("broken product id")?.as_u64().ok_or("broken u64")?);
            }

        }

        (map_product, map_service)
    };
    // stock_product, stock_service
    {
        let sql = r#"
        select
        prc_venda as "value",
        prc_custo as value_cost,
        margem_lb as margin_sale,
        id_estoque as id_import
        from tb_estoque order by id_import
        "#;

        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let mut obj = get_json(&row?)?;
            let id_import = obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?;

            if let Some(id) = map_service.get(&id_import) {
                obj["id"] = json!(id);
                match rufs.entity_manager.insert(&rufs.openapi, "stock_service", &obj).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                }
            } else if let Some(id) = map_product.get(&id_import) {
                obj["id"] = json!(id);
                match rufs.entity_manager.insert(&rufs.openapi, "stock_product", &obj).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                };
            }
        }
    }

    let _map_request_repair = {
        let sql = r#"
        select
        defeito as defect,
        coalesce(ident1,'') as serial,
        ident2 as inmetro,
        ident3 as marca,
        ident4 as modelo,
        ident5 as capacidade,
        ('Marca: ' || coalesce(ident3,'') || '\nModelo: ' || coalesce(ident4,'') || '\nCapacidade: ' || coalesce(ident5,'')) as additional_data,
        id_os as id_import
        from tb_os_objeto_os order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
        let mut map = HashMap::new();

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_repair import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let id_os = obj.get("idImport").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")? * 10000;

            let Some(request) = map_request.get(&id_os) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] request_repair : Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            let Some(product) = map_product.get(&1) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing product in map_product.")?;
            };

            obj["product"] = json!(product);
            
            let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_repair", &obj).await {
                Ok(value) => value,
                Err(err) => {
                    println!("[import_clipp] request import insert : {}", err);
                    return Err(err)?;
                },
            };

            map.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out.get("request").ok_or("broken request id in request repair.")?.as_u64().ok_or("broken u64")?);
        }

        map
    };

    let (mut map_request_product, _map_request_service) = {
        let sql = r#"
        select
        id_identificador,
        id_os,
        qtd_item as quantity,
        vlr_unit as "value",
        vlr_desc as value_desc,
        vlr_total as value_item,
        id_itemos as id_import
        from tb_os_item order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;
        let mut map_request_product = HashMap::new();
        let mut map_request_service = HashMap::new();

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_product import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let id_identificador = obj.get("idIdentificador").ok_or("Missing idIdentificador")?.as_u64().ok_or("broken u64 idIdentificador")?;
            let id_os = obj.get("idOs").ok_or("Missing idOs")?.as_u64().ok_or("broken u64 idOs")? * 10000;

            let Some(request) = map_request.get(&id_os) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            if let Some(product) = map_product.get(&id_identificador) {
                obj["product"] = json!(product);

                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_product", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_product_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };
    
                map_request_product.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else if let Some(service) = map_service.get(&id_identificador) {
                obj["service"] = json!(service);

                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_service", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_service_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };
    
                map_request_service.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("idIdentificador is not product or service")?;
            }
        }

        (map_request_product, map_request_service)
    };

    {
        let sql = r#"
        select
        id_nfvenda,
        id_identificador,
        cfop,
        qtd_item as quantity,
        vlr_unit as "value",
        vlr_desc as value_desc,
        vlr_total as value_item,
        vlr_frete as value_freight,
        vlr_tottrib as value_all_tax,
        id_nvfitem as id_import
        from tb_nfv_item order by id_import
        "#;
        let rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

        for row in rows {
            let row = match row {
                Ok(row) => row,
                Err(err) => {
                    println!("[import_clipp] request_product import select : {}", err);
                    return Err(err)?;
                },
            };

            let mut obj = get_json(&row)?;
            let id_identificador = obj.get("idIdentificador").ok_or("Missing idIdentificador")?.as_u64().ok_or("broken u64 idIdentificador")?;
            let id_nfvenda = obj.get("idNfvenda").ok_or("Missing idNfvenda")?.as_u64().ok_or("broken u64 idNfvenda")?;

            let Some(request) = map_request.get(&id_nfvenda) else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("[import_clipp] Missing id_os in map_request.")?;
            };

            obj["request"] = json!(request);

            if let Some(product) = map_product.get(&id_identificador) {
                obj["product"] = json!(product);

                let obj_out = match rufs.entity_manager.insert(&rufs.openapi, "request_product", &obj).await {
                    Ok(value) => value,
                    Err(err) => {
                        if err.to_string().contains(r#"duplicate key value violates unique constraint "request_product_pkey""#) {
                            continue;
                        }

                        return Err(err)?;
                    },
                };
    
                map_request_product.insert(obj.get("idImport").ok_or("broken id_import")?.as_u64().ok_or("broken u64")?, obj_out);
            } else {
                println!("{}", serde_json::to_string_pretty(&obj)?);
                return Err("idIdentificador is not product")?;
            }
        }

    }

    {
        let sql = r#"
        select
        a.id_nfvenda,
        r.documento, 
        r.dt_vencto,
        r.vlr_ctarec,
        r.id_ctarec as id_import,
        from tb_conta_receber r,tb_nfv_ctarec a where r. order by id_import
        "#;
        let _rows: Box<dyn Iterator<Item = Result<rsfbclient::Row, FbError>>> = fb_conn.query_iter(sql, ())?;

    }
/*
TB_NFVENDA_FMAPAGTO_NFCE
TB_NFV_CTAREC
TB_CONTA_RECEBER
*/
    Ok(())
}

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

    println!("std::env::current_dir() : {:?}", std::env::current_dir()?);
    println!("fs_prefix = {:?}", fs_prefix);
    let db_uri = RufsMicroService::build_db_uri(None, None, None, None, Some(&params.app_name), None);
    println!("db_uri = {}", db_uri);
    let mut rufs = RufsMicroService::connect(&db_uri, &format!("{}sql", fs_prefix), params, &WATCHER).await?;

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

    #[cfg(debug_assertions)]
    #[cfg(feature = "clipp")]
    import_clipp(&rufs).await?;

    #[cfg(feature = "warp")]
    #[cfg(not(feature = "tide"))]
    {
        use warp::Filter;

        let rufs_routes = rufs_base_rust::rufs_micro_service::rufs_warp(rufs, &RUFS_STATE.authenticator).await;
        let listener = format!("127.0.0.1:{}", args.port);
        println!("Staring rufs-nfe server at {}", listener);
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
            //.with(cors)
            )
            ;
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
        println!("...server exited with error : {}", err);
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
