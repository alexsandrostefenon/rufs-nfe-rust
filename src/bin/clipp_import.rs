use reqwest::StatusCode;
use serde_json::{Value, json};

async fn import(message: &Value) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use convert_case::Casing;
    use rsfbclient::{prelude::*, FbError};

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

	fn convert_obj_copy(obj_out: &mut Value, obj_in :&Value, primary_keys :&[&str]) -> Result<(), Box<dyn std::error::Error>> {
		for (field_name_in, field) in obj_in.as_object().unwrap() {
			let field_name_out = field_name_in.to_case(convert_case::Case::Camel);

			match field {
				Value::Object(_obj) => continue,
				Value::Null => continue,
				Value::String(value) => {
					if value.is_empty() && primary_keys.contains(&field_name_in.as_str()) == false {
						continue;
					}
				},
				_ => {},
			}

			obj_out[field_name_out] = field.clone();
		}

		Ok(())
	}

	fn convert_obj(obj_in :&Value, primary_keys :&[&str]) -> Result<Value, Box<dyn std::error::Error>> {
		let mut obj_out = json!({});
		convert_obj_copy(&mut obj_out, obj_in, primary_keys)?;
		Ok(obj_out)
	}

	#[cfg(debug_assertions)]
	let url_base = format!("http://localhost:8080/nfe/rest/");
	#[cfg(not(debug_assertions))]
	let url_base = format!("http://rufs-nfe:8080/rest/");
	let client = reqwest::Client::new();
	let token = message.get("authorization").ok_or("Missing field 'authorization' in message")?.as_str().ok_or("Broken field 'authorization' in message")?;

	let post = async |schema_name_snake: &str, obj: &Value| -> Result<Value, Box<dyn std::error::Error>> {
		let url = format!("{url_base}{schema_name_snake}");
		#[cfg(debug_assertions)]
		println!("[nfe_import.post({schema_name_snake})] {url}");
		let res = client.post(url).bearer_auth(token).json(obj).send().await?;
		let status = res.status();

		if status == StatusCode::OK {
			return Ok(res.json().await?);
		} else {
			let text = res.text().await?;
			return Err(text)?;
		}
	};

	let publish = async |schema_name_snake :&str, primary_keys :&[&str], rec_imported :&Value, replace: bool| -> Result<Value, Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		println!("[nfe_import.publish({schema_name_snake})] starting ...");
		#[cfg(debug_assertions)]
		println!("[nfe_import.convert_obj()] starting ...");
		let obj = convert_obj(rec_imported, primary_keys)?;
		#[cfg(debug_assertions)]
		println!("[nfe_import.convert_obj()] ... exited");
		let mut query_list = vec![];

		for primary_key_name in primary_keys {
			#[cfg(debug_assertions)]
			println!("[nfe_import.primary_key_name {primary_key_name}] starting ...");
			let Some(primary_key) = obj.get(primary_key_name) else {
				let str = serde_json::to_string_pretty(&obj).unwrap();
				eprintln!("[nfe_import.merge.primary_key_name] error : {str}.");
				return Err(format!("Missing field '{primary_key_name}' in structure {schema_name_snake} in parsed object : {str}"))?;
			};

			let primary_key = match primary_key {
				Value::Number(number) => number.to_string(),
				Value::String(value) => value.to_string(),
				_ => return Err(format!("Invalid primary_key : {primary_key}"))?,
			};

			query_list.push(format!("{primary_key_name}={primary_key}"));
			#[cfg(debug_assertions)]
			println!("[nfe_import.primary_key_name {primary_key_name}] ... exited");
		}

		let url = format!("{url_base}{schema_name_snake}?{}", query_list.join("&"));

		#[cfg(debug_assertions)]
		println!("[nfe_import.put] {url}, starting ...");
		let method;
		let res = if replace {
			method = "put";
			client.put(&url).bearer_auth(token).json(&obj).send().await?
		} else {
			method = "patch";
			client.patch(&url).bearer_auth(token).json(&obj).send().await?
		};
		#[cfg(debug_assertions)]
		println!("[nfe_import.put] ... exited");

		let status = res.status();

		let obj = if status != StatusCode::OK {
			let text = res.text().await?;

			if text.starts_with("Missing data in") {
				post(schema_name_snake, &obj).await?
			} else {
				eprintln!("[nfe_import.merge.publish()]\ncurl -X {method} {url} -d '{obj}';\nerror message : {text}");
				return Err(text)?;
			}
		} else {
			let value = res.json::<Value>().await?;

			if let Some(list) = value.as_array() {
				if list.len() == 0 {
					post(schema_name_snake, &obj).await?
				} else {
					return Err(format!("Bad put/patch response : {:?}", list))?;
				}
			} else {
				value
			}
		};

		#[cfg(debug_assertions)]
		println!("[nfe_import.publish({schema_name_snake})] ... finished");
		return Ok(obj);
	};

	let db_url = message.get("dbUrl").ok_or("Missing field 'dbUrl' in message")?.as_str().ok_or("Broken field 'dbUrl' in message")?;
    let mut fb_conn = rsfbclient::builder_pure_rust().from_string(db_url)?.connect()?;

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

            let _obj_out = match publish("person", &["cnpjCpf"], &obj, false).await {
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

            let obj_out = match publish("request", &["id"], &obj, false).await {
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

            let _obj_out = match post("request_nfe", &obj).await {
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

            let obj_out = match publish("request", &["id"], &obj, false).await {
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
                let obj_out = match publish("service", &["id"], &obj, false).await {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(err)?;
                    },
                };

                map_service.insert(id_import, obj_out.get("id").ok_or("broken service id")?.as_u64().ok_or("broken service id u64")?);
            } else {
                let obj_out = match publish("product", &["id"], &obj, false).await {
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
                match publish("stock_service", &["id"], &obj, false).await {
                    Ok(_value) => {},
                    Err(err) => {
                        return Err(err)?;
                    },
                }
            } else if let Some(id) = map_product.get(&id_import) {
                obj["id"] = json!(id);
                match publish("stock_product", &["id"], &obj, false).await {
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

            let obj_out = match publish("request_repair", &["id"], &obj, false).await {
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

                let obj_out = match publish("request_product", &["id"], &obj, false).await {
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

                let obj_out = match publish("request_service", &["id"], &obj, false).await {
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

                let obj_out = match publish("request_product", &["id"], &obj, false).await {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "kafka"))]
	{
        let host = std::env::var("REDIS_HOST").unwrap_or("127.0.0.1".to_owned());
        let client = redis::Client::open(format!("redis://{host}/")).map_err(|err| format!("Redis failt : {err}"))?;
		let mut con = client.get_connection().expect("conn");

		const GROUP_NAME: &str = "clipp_group";
		let res: Result<(), _> = redis::Commands::xgroup_create_mkstream(&mut con, "clipp", GROUP_NAME, "$");

		if let Err(e) = res {
			println!("[clipp_import.main] Group already exists: {e:?}")
		}

		let opts = redis::streams::StreamReadOptions::default().block(10000).group(GROUP_NAME, "clipp-import-1");

		loop {
			let srr: Result<redis::streams::StreamReadReply, redis::RedisError> = redis::Commands::xread_options(&mut con,
				&["clipp"],
				&[">"],
				&opts);

			let srr = match srr {
				Ok(srr) => srr,
				Err(err) => {
					eprintln!("[clipp_import.main] {err}");
					break;
				},
			};

			for redis::streams::StreamKey { key, ids } in srr.keys {
				println!("[clipp_import.main] Stream key {key}");
				let mut list_id = vec![];

				for redis::streams::StreamId { id, map } in ids {
					println!("[clipp_import.main] Redis Events Stream : ID {id}");

					for (n, s) in map {
						println!("[clipp_import.main] Redis Events Stream : Stream n {n}");

						if let redis::Value::BulkString(bytes) = s {
							let message = std::str::from_utf8(&bytes);

							match message {
								Ok(message) => {
									println!("[clipp_import.main] Redis Events Stream : {message}.");
									let value = serde_json::from_str::<Value>(message);

									match value {
											Ok(value) => {                                                
												if let Err(err) = import(&value).await {
													eprintln!("[clipp_import.main] Redis Events Stream : Error of process_broker_message({message}) : {}", err);
												}
											},
											Err(err) => eprintln!("[clipp_import.main] Redis Events Stream : value is not valid json ({message}) : {}", err),
										}
								},
								Err(err) => eprintln!("[clipp_import.main] Redis Events Stream : value is not utf8 string : {}", err),
							}
						} else {
							panic!("Weird data")
						}
					}

					list_id.push(id);
				}

				redis::TypedCommands::xack(&mut con, key, GROUP_NAME, &list_id).expect("ack");
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    #[tokio::test]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
		let list = [
            // WIN1252,ISO8859_1
            //"firebird://SYSDBA:masterkey@localhost:3050//var/lib/firebird/data/CLIPP-3.fdb?charset=WIN1252",
            "firebird://SYSDBA:masterkey@localhost:3050//var/lib/firebird/data/CLIPP-2024_09_14.fdb?charset=WIN1252"
		];

		let customer_user = "12345678901.guest";
		let password = "e10adc3949ba59abbe56e057f20f883e";
		let client = reqwest::Client::new();
		let res = client.post("http://localhost:8080/nfe/rest/login").json(&json!({"user": customer_user, "password": password})).send().await?;
		let status = res.status();

		let login_response = if status == reqwest::StatusCode::OK {
			let res: Value = res.json().await?;
			res
		} else {
			let text = res.text().await?;
			return Err(text)?;
		};

		let mut message = json!({});
		message["authorization"] = login_response.get("jwtHeader").ok_or("Missing jwtHeader in login response!")?.clone();

		for item in list {
			message["dbUrl"] = json!(item);
			crate::import(&message).await?;
		}

        Ok(())
    }
}
