use std::collections::VecDeque;
use chrono::{DateTime, Months};
use convert_case::Casing;
use regex::Regex;
use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio::{fs::File, io::AsyncReadExt};

async fn merge(file_path :&str, message :&Value, document_imported :&Value) -> Result<Value, Box<dyn std::error::Error>> {
/*
			item.taxGroup.ncm = ncm.data.id;
			item.taxGroup.city = personEmitente.data.city;
			const taxGroup = await serverConnection.services.nfeTaxGroup.patch(item.taxGroup);
*/
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

	fn compare_request_product(a: &Value, b: &Value) -> bool {
		let ret = (a.get("product") == b.get("product")) && (a.get("serials") == b.get("serials"));
		ret
	}

	#[cfg(debug_assertions)]
	let host = "localhost";
	#[cfg(not(debug_assertions))]
	let host = "rufs-nfe";
	let port = 8080;
	#[cfg(debug_assertions)]
	let url_base = format!("http://{host}:{port}/nfe/rest/");
	#[cfg(not(debug_assertions))]
	let url_base = format!("http://{host}:{port}/rest/");
	let client = reqwest::Client::new();

	let Some(token) = message.get("authorization") else {
		return Err(format!("Missing field ath in mssag"))?;
	};

	let Some(token) = token.as_str() else {
		return Err(format!("Field ath is not string."))?;
	};

	let post = async |schema_name_snake: &str, obj: &Value| -> Result<Value, Box<dyn std::error::Error>> {
		let url = format!("{url_base}{schema_name_snake}");
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

	let person = document_imported.get("person").ok_or("Broken person")?;
	publish("person", &["cnpjCpf"], &person, false).await?;
	let person_cnpj_cpf = person.get("cnpj_cpf").ok_or("Missing cnpj_cpf in person")?.as_str().ok_or("Bad type")?;
	let mut person_dest = document_imported.get("person_dest").ok_or("Broken person dest")?.clone();

	if person_dest.get("cnpj_cpf").is_none() {
		let regex = Regex::new(r#"(\d{8,14})"#)?;
		let s = regex.find(file_path).ok_or("Broken customer_id")?.as_str();

		let cnpj_cpf = if s.len() > 11 {
			let s = format!("{:014}", s);
			format!("{}.{}.{}/{}-{}", &s[0..2], &s[2..5], &s[5..8], &s[8..12], &s[12..14])
		} else {
			let s = format!("{:011}", s);
			format!("{}.{}.{}-{}", &s[0..3], &s[3..6], &s[6..9], &s[9..11])
		};

		person_dest["cnpj_cpf"] = json!(cnpj_cpf);
	}

	publish("person", &["cnpjCpf"], &person_dest, false).await?;
	let person_dest_cnpj_cpf = person_dest.get("cnpj_cpf").ok_or("Missing cnpj_cpf in person_dest")?.as_str().ok_or("Bad type")?;
	let request_nfe_snake = document_imported.get("request_nfe").ok_or("Missing structure request_nfe in parsed object")?;
	let mut request_nfe = convert_obj(request_nfe_snake, &[])?;
	convert_obj_copy(&mut request_nfe, request_nfe_snake.get("emissao").ok_or("Missing 'emissao'")?, &[])?;
	convert_obj_copy(&mut request_nfe, request_nfe_snake.get("destinatario").ok_or("Missing 'destinatario'")?, &[])?;
	convert_obj_copy(&mut request_nfe, document_imported.get("informacoes_adicionais").ok_or("Missing 'informacoes_adicionais'")?, &[])?;
	convert_obj_copy(&mut request_nfe, document_imported.get("totais").ok_or("Missing 'totais'")?, &[])?;

	let nfe_id = request_nfe.get("nfeId").ok_or("Missing field nfeId")?.as_str().ok_or("Expected 'string' type")?;
	let url = format!("{url_base}request_nfe?filter[nfeId]={nfe_id}");
	#[cfg(debug_assertions)]
	println!("[nfe_import.merge] curl -X get '{url}' -H 'Authorization: Bearer {token}';");
	let res = client.get(&url).bearer_auth(token).send().await?;
	let status = res.status();

	if status == StatusCode::OK {
		let list = res.json::<Value>().await?;
		let list = list.as_array().ok_or("Bad format, expected list!")?;

		if list.len() > 0 {
			let ret = list.get(0).ok_or("broken")?;
			println!("[nfe_import.merge] NFE {nfe_id} already processed, exiting !");
			return Ok(ret.clone());
		}
	} else {
		return Err(res.text().await?)?;
	}

	let request_type = request_nfe.get("type").ok_or("Missing field 'type' in 'request_nfe'")?.as_u64().ok_or("field 'type' is not 'u64'")?;
	let date = request_nfe.get("date").ok_or("Missing field 'date' in 'request_nfe'")?.as_str().ok_or("field 'date' is not 'str'")?.to_string()+"+03:00";
	let date_min = DateTime::parse_from_rfc3339(&date)?.checked_sub_months(Months::new(1)).ok_or("broken 'date_min'")?.to_rfc3339();
	//let date_max = DateTime::parse_from_rfc3339(&date)?.checked_add_days(Days::new(1)).ok_or("broken")?.to_rfc3339();
	let url = format!("{url_base}request?filter[person]={person_cnpj_cpf}&filter[personDest]={person_dest_cnpj_cpf}&filter[type]={request_type}&filter[paymentsValue]=0&filterRangeMin[date]={date_min}&filterRangeMax[date]={date}");
	#[cfg(debug_assertions)]
	println!("[nfe_import.merge] curl -X get '{url}' -H 'Authorization: Bearer {token}';");
	let res = client.get(&url).bearer_auth(token).send().await?;
	let status = res.status();

	let mut request = if status == StatusCode::OK {
		let list = res.json::<Value>().await?;
		let list = list.as_array().ok_or("Bad format, expected list!")?;

		match list.len() {
			0 => {
				request_nfe["person"] = json!(person_cnpj_cpf);
				request_nfe["personDest"] = json!(person_dest_cnpj_cpf);
				request_nfe["state"] = json!(80);
				post("request", &request_nfe).await?
			},
			_ => list.last().ok_or("Broken")?.clone(),
		}
	} else {
		return Err(res.text().await?)?;
	};

	let request_id = request.get("id").ok_or("broken")?.clone();
	request_nfe["request"] = request_id.clone();

	if let Some(date_in_out) = request_nfe.get("dateInOut") {
		if date_in_out.as_str().ok_or("broken")?.is_empty() {
			request_nfe["dateInOut"] = request_nfe.get("date").ok_or("broken")?.clone();
		}
	} else {
		request_nfe["dateInOut"] = request_nfe.get("date").ok_or("broken")?.clone();
	}

	let list = document_imported.get("request_product").ok_or("Broken request_product")?.as_array().ok_or("expected array")?;
	let mut list_to_publish = vec![];

	fn add_f64(item_out: &mut Value, name_out: &str, item_in: &Value, name_in: &str) -> Result<f64, Box<dyn std::error::Error>> {
		let value_in = if let Some(value) = item_in.get(name_in) {
			value.as_f64().ok_or("broken value as f64")?
		} else {
			0.0
		};

		let value_out = if let Some(value_to_publish) = item_out.get(name_out) {
			let value_to_publish = value_to_publish.as_f64().unwrap_or_default() + value_in;
			item_out[name_out] = json!(value_to_publish);
			value_to_publish
		} else {
			item_out[name_out] = json!(value_in);
			value_in
		};

		Ok(value_out)
	}

	for request_product in list {
		if request_product.get("quantity").is_none() {
			continue;
		}

		let mut request_product = convert_obj(request_product, &[])?;
		request_product["request"] = request.get("id").ok_or("broken")?.clone();
		request_product["serials"] = json!("");
/*
		if let Some(id_import) = request_product.get("idImport") {
			if let Some(id_import) = id_import.as_str() {
				if id_import
				request_product["id_import"] = json!(null);
			}
		}
*/
		let mut product_id = 0;
		let mut do_patch_product = true;
		let mut barcode_valid = false;

		if let Some(barcode) = request_product.get("barcode") {
			let regex_barcode = Regex::new(r#"^\d{1,14}$"#)?;
			let barcode = barcode.as_str().ok_or("Brokn barcode typ")?;

			if regex_barcode.is_match(barcode) {
				barcode_valid = true;
				let url = format!("{url_base}barcode?barcode={barcode}");
				#[cfg(debug_assertions)]
				println!("[nfe_import.merge] curl -X get '{url}' -H 'Authorization: Bearer {token}';");
				let res = client.get(&url).bearer_auth(token).send().await?;
				let status = res.status();

				if status == StatusCode::OK {
					let list = res.json::<Value>().await?;
					let list = list.as_array().ok_or("Bad format, expected list!")?;

					if list.len() > 0 {
						product_id = list.get(0).ok_or("Broken")?.get("product").ok_or("Missing field 'product'")?.as_u64().ok_or("'product' not 'u64'")?;
						request_product["product"] = json!(product_id);
					}
				} else {
					return Err(res.text().await?)?;
				}
			}
		}

		if product_id == 0 {
			let name = request_product.get("name").ok_or("Missing field 'name'")?.as_str().ok_or("'product' not 'str'")?;
			let url = format!("{url_base}product?filter[name]={name}");
			#[cfg(debug_assertions)]
			println!("[nfe_import.merge] curl -X get '{url}' -H 'Authorization: Bearer {token}';");
			let res = client.get(&url).bearer_auth(token).send().await?;
			let status = res.status();

			if status == StatusCode::OK {
				let list = res.json::<Value>().await?;
				let list = list.as_array().ok_or("Bad format, expected list!")?;

				if list.len() > 0 {
					product_id = list.get(0).ok_or("Broken")?.get("id").ok_or("Missing field 'product'")?.as_u64().ok_or("'id' not u64")?;
					request_product["product"] = json!(product_id);
				}
			} else {
				return Err(res.text().await?)?;
			}

			if product_id == 0 {
				do_patch_product = false;
				product_id = post("product", &request_product).await?.get("id").ok_or("Missing field 'id'")?.as_u64().ok_or("'id ' is not u64")?;
				request_product["product"] = json!(product_id);
			}

			if barcode_valid {
				let _barcode = publish("barcode", &["barcode"], &request_product, false).await?;
			}
		}

		if do_patch_product {
			let url = format!("{url_base}product?id={product_id}");
			#[cfg(debug_assertions)]
			println!("[nfe_import.merge] curl -X patch '{url}' -H 'Authorization: Bearer {token}';");
			let res = client.patch(url).bearer_auth(token).json(&request_product).send().await?;

			if res.status() != StatusCode::OK {
				return Err(res.text().await?)?;
			}
		}

		if let Some(item_to_publish) = list_to_publish.iter_mut().find(|x| compare_request_product(x, &request_product)) {
			add_f64(item_to_publish, "quantity", &request_product, "quantity")?;
			add_f64(item_to_publish, "valueItem", &request_product, "valueItem")?;
			add_f64(item_to_publish, "valueDesc", &request_product, "valueDesc")?;
			add_f64(item_to_publish, "valueFreight", &request_product, "valueFreight")?;
			item_to_publish["merged"] = json!(true);
		} else {
			list_to_publish.push(request_product);
		}
	}

	let mut value_products = 0.0;

	for request_product in list_to_publish.iter_mut() {
		let value_desc = if let Some(value) = request_product.get("valueDesc") {
			value.as_f64().ok_or("broken valieItem f64")?
		} else {
			0.0
		};

		let value_item = request_product.get("valueItem").ok_or("broken valueItem")?.as_f64().ok_or("broken valieItem f64")?;
		value_products += value_item - value_desc;
		#[cfg(debug_assertions)]
		println!("value_products {value_products} += value_item {value_item} - value_desc {value_desc};");

		if request_product.get("merged").is_none() == false {
			let quantity = request_product.get("quantity").ok_or("broken quantity")?.as_f64().ok_or("broken quantity f64")?;

			if quantity != 0.0 {
				let value = (value_item * 100.0) / quantity;
				request_product["value"] = json!(value.round() / 100.0);
			}
		}

		let _item = publish("request_product", &["request", "product", "serials"], &request_product, true).await?;
	}

	let map_accounts = std::collections::HashMap::from([
		(1, "Caixa interno"),
		(2, "Conta corrente"),
		(3, "Cartão de Crédito"),
		(4, "Conta corrente"),
		(5, "Caixa interno"),
		(10, "Vale Alimentação"),
		(11, "Vale Refeição"),
		(12, "Vale Presente"),
		(13, "Vale Combustível"),
		(14, "Conta corrente"),
		(15, "Conta corrente"),
		(17, "Conta corrente"),
		(20, "Conta corrente"),
	]);

	let list = document_imported.get("request_payment").ok_or("Missing field 'request_payment'.")?.as_array().ok_or("'request_payment' is not array.")?;
	let mut value_payments = 0.0;
	let mut count_sem_pagamento = 0;

	for request_payment in list {
		let account = {
			let payment_type = request_payment.get("type").ok_or("Missing field 'type'.")?.as_u64().ok_or("field 'type' is not 'u64'")?;
			let description = map_accounts.get(&payment_type).unwrap_or(&"Caixa interno");
			let url = format!("{url_base}account?person={person_dest_cnpj_cpf}&description={description}");
			#[cfg(debug_assertions)]
			println!("[nfe_import.merge] curl -X get '{url}' -H 'Authorization: Bearer {token}';");
			let res = client.get(&url).bearer_auth(token).send().await?;

			if res.status() == StatusCode::OK {
				let list = res.json::<Value>().await?;
				let list = list.as_array().ok_or("Bad format, expected list!")?;

				if list.len() > 0 {
					list.get(0).ok_or("broken request_payment list index 0")?.clone()
				} else {
					let account = json!({"person": person_dest_cnpj_cpf, "description": description});
					#[cfg(debug_assertions)]
					println!("[nfe_import.merge] curl -X post '{url}' -H 'Authorization: Bearer {token}';");
					let res = client.post(url_base.clone() + "account").bearer_auth(token).json(&account).send().await?;
					let status = res.status();

					if status == StatusCode::OK {
						res.json::<Value>().await?
					} else {
						return Err(res.text().await?)?;
					}
				}
			} else {
				return Err(res.text().await?)?;
			}
		};

		let mut request_payment = convert_obj(request_payment, &[])?;
		request_payment["account"] = account.get("id").ok_or("Missing field 'id' in account")?.clone();
		request_payment["request"] = request.get("id").ok_or("broken")?.clone();
		request_payment["dueDate"] = json!(date);
		post("request_payment", &request_payment).await?;

		value_payments += if let Some(value) = request_payment.get("value") {
			value.as_f64().ok_or("broken value as f64")?
		} else {
			0.0
		};

		if let Some(typ) = request_payment.get("type") {
			if let Some(typ) = typ.as_u64() {
				if typ == 90 {
					count_sem_pagamento += 1;
				}
			}
		}
	}

	request["paymentsValue"] = json!(value_payments);
	let mut request_freight = document_imported.get("request_freight").ok_or("Broken request_freight")?.clone();

	if let Some(value_frete) = request.get("transportValue") {
		if request_freight.get("value").is_none() {
			request_freight["value"] = value_frete.clone();
		}
	}

	let transport_value = request_freight.get("value").ok_or("Broken request_freight.value")?.as_f64().unwrap_or(0.0);

	if transport_value > 0.0 {
		request["transportValue"] = json!(transport_value);
		request_freight["value"] = json!(transport_value);
		request_freight["request"] = request_id;
		publish("request_freight", &["request"], &request_freight, true).await?;
	}

	publish("request", &["id"], &request, true).await?;

	if count_sem_pagamento == 0 || count_sem_pagamento != list.len() {
		if f64::round(value_payments * 100.0) != f64::round(value_products * 100.0 + transport_value * 100.0) {
			return Err(format!("value_products ({value_products}) !=  value_payments ({value_payments}) !"))?;
		}
	}

	post("request_nfe", &request_nfe).await
}

async fn parse_html(html: &str) -> Result<Value, Box<dyn std::error::Error>> {
	fn get_uf_code(uf :&str) -> usize {
		match uf {
			"RS" => 43,
			"SP" => 35,
			"SC" => 42,
			"PR" => 41,
			"DF" => 53,
			"ES" => 32,
			"RJ" => 33,
			_ => 0
		}
	}

	fn label_to_name(label :&str) -> String {
		let label = unaccent::unaccent(label)
			.replace("\n", " ")
			.replace("&nbsp;", " ")
			.replace("<i>", " ")
			.replace("</i>", " ")
			.replace(" e ", " ")
			.replace(" da ", " ")
			.replace(" de ", " ")
			.replace(" do ", " ")
			.replace(" dos ", " ")
			.replace("-", " ")
			.replace(":", " ")
			.replace(".", " ")
			.replace("(", " ")
			.replace(")", " ")
			.replace("/", " ")
			.replace("    ", " ")
			.replace("   ", " ")
			.replace("  ", " ")
			.trim()
			.replace(" ", "_")
			.to_lowercase()
			.replace("valor", "value");
		let name = match label.as_str() {
			"dados_nf_e" => "request_nfe",
			//"emissao" => "request_nfe",
			//"destinatario" => "request_nfe",
			//"informacoes_adicionais" => "request_nfe",
			//"informacoes_suplementares" => "request_nfe",
			//"icms" => "request_nfe",
			//"totais" => "request_nfe",
			"dados_emitente" => "person",
			//"emitente" => "person",
			"dados_destinatario" => "person_dest",
			"dados_produtos_servicos" => "request_product",
			"formas_pagamento" => "request_payment",
			"dados_transporte" => "request_freight",
			"cnpj" => "cnpj_cpf",
			"cpf" => "cnpj_cpf",
			"nome_razao_social" => "name",
			"inscricao_estadual" => "ie_rg",
			"nome_fantasia" => "fantasy",
			"endereco" => "address",
			"bairro_distrito" => "district",
			"cep" => "zip",
			"municipio" => "city",
			"telefone" => "phone",
			"pais" => "country",
			"inscricao_municipal" => "im",
			"cnae_fiscal" => "cnae",
			"codigo_regime_tributario" => "crt",
			"data_emissao" => "date",
			"data_saida_entrada" => "date_in_out",
			"value_total_nota_fiscal" => "sum_value",
			"destino_operacao" => "id_dest",
			"consumidor_final" => "ind_final",
			"presenca_comprador" => "ind_pres",
			"inscricao_suframa" => "suframa",
			"e_mail" => "email",
			"indicador_ie" => "ind_ie_dest",
			"codigo_ncm" => "ncm",
			"codigo_cest" => "cest",
			"codigo_ean_tributavel" => "barcode",
			"unidade_tributavel" => "unity",
			"quantidade_tributavel" => "quantity",
			"value_unitario_tributacao" => "value",
			"fixo_prod_serv_descricao" => "name",
			"fixo_prod_serv_vb" => "value_item",
			"codigo_produto" => "id_import",
			"origem_mercadoria" => "orig",
			"tributacao_icms" => "cst_icms",
			"value_pagamento" => "value",
			"meio_pagamento" => "type",
			"modalidade_frete" => "pay_by",
			"formato_impressao_danfe" => "tp_imp",
			"modelo" => "mod",
			"processo" => "proc_emi",
			"versao_processo" => "ver_proc",
			//"tipo_emissao" => "fin_nfe",
			"finalidade" => "fin_nfe",
			"natureza_operacao" => "nat_op",
			"tipo_operacao" => "type",
			"value_total_descontos" => "desc_value",
			"value_total_ii" => "value_ii",
			"value_total_ipi" => "value_ipi",
			"value_aproximado_tributos" => "value_tax",
			"value_total_produtos" => "products_value",
			"value_frete" => "transport_value",
			"value_total_frete" => "value_freight",
			"value_desconto" => "value_desc",
			"value_icms_substituicao" => "value_icms_st",
			"digest_value_nf_e" => "digest",
			"qr_code" => "nfe_id",
			"chave_acesso_dfe" => "nfe_id",
			"" => "",
			_ => &label,
		};
	/*
	"base_calculo_icms": "65,89",
	"valor_total_nfe": "159,87",
	"indicador_intermediador_marketplace": "0 - Operação sem intermediador",
	"digest_value_nf_e": "CbxWV71Bxz5aLVHcuEsJqlNP2gA=",
	"municipio_ocorrencia_fato_gerador_icms": "4309050",
	"indicador_escala_relevante": "S - Produzido em Escala Relevante;",
	"indicador_composicao_valor_total_nf_e": "1 - O valor do item (vProd) compõe o valor total da NF-e (vProd)",
	"modalidade_definicao_bc_icms_normal": "3 - Valor da Operação",
	"base_calculo_icms_normal": "3,49",
	"aliquota_icms_normal": "17,0000",
	"valor_icms_normal": "0,59",
	*/
		name.to_string()
	}

	fn get_value(table :&str, field :&str, text: &str) -> Result<Value, Box<dyn std::error::Error>> {
		fn parse_date(text :&str) -> String {
			let regex = Regex::new(r#"(?P<dia>\d{1,2})/(?P<mes>\d{1,2})/(?P<ano>\d{4}) (?P<hora>\d{1,2}:\d{1,2}:\d{1,2}).*"#).unwrap();
			regex.replace_all(text, "$ano-$mes-${dia}T$hora").to_string()
		}

		fn parse_number(text :&str) -> Result<f64, Box<dyn std::error::Error>> {
			let replacement = text.replace(".", "").replace(",", ".");
			let value = replacement.as_str().parse::<f64>()?;
			Ok(value)
		}

		fn parse_id(table: &str, field: &str, value: &str) -> Result<Value, Box<dyn std::error::Error>> {
			let regex = Regex::new(r#"^(?P<id>\d+)"#).unwrap();

			let Some(res) = regex.captures(&value) else {
				Err(format!(r"Invalid ID (^(?P<id>\d+)) content of {table}.{field} : '{value}'"))?
			};

			let res = res.name("id").unwrap().as_str();

			if res.len() > 8 {
				eprintln!("[nfe_import.parse_html.get_value_.parse_id] Ivalid ID value size for {table}.{field} : {value}");
				return Ok(Value::Null);
			}

			let value = res.parse::<usize>()?;

			let value = match (table, field, value) {
				(_, _, 0) => Value::Null,
				_ => json!(value)
			};

			Ok(value)
		}

		fn parse_reg_ex_match(_table: &str, _field: &str, reg_ex: &str, value: &str, typ: Value, default: Value) -> Result<Value, Box<dyn std::error::Error>> {
			let regex = regex::Regex::new(reg_ex)?;//.case_insensitive(true).build()?;

			let Some(res) = regex.find(&value) else {
				return Ok(default);
			};

			if res.is_empty() {
				return Ok(default);
			}

			let ret = match typ {
				Value::Number(number) => {
					if number.is_u64() {
						json!(res.as_str().parse::<usize>()?)
					} else {
						json!(res.as_str().parse::<f64>()?)
					}
				},
				_ => json!(res.as_str().to_uppercase()),
			};

			Ok(ret)
		}

		let mut value = text.trim().to_uppercase();

		if value.ends_with(" NULL") {
			value = value[0..value.len()-5].to_string();
		}

	/*
				if (person.cnpjCpf) person.cnpjCpf = person.cnpjCpf.replace(/\D/g,'');
				if (person.ieRg) person.ieRg = person.ieRg.replace(/\D/g,'');
				if (person.zip) person.zip = person.zip.replace(/\D/g,'');
				if (person.phone) person.phone = person.phone.replace(/\D/g,'');
	*/
		let value = match field {
			//"chave_acesso" => value.replace(" ", ""),
			"uf" => get_uf_code(&value).to_string(),
			"nfe_id" => {
				let regex_nfe_id = Regex::new(r#".*(?P<id>\d{44,47}).*"#).unwrap();
				regex_nfe_id.replace_all(&value, "$id").to_string()
			}
			_ => value,
		};

		let mut value = match (field, value.as_str()) {
			("country", "BRASIL") => "1058".to_string(),
			("city", "GLORINHA") => "4309050".to_string(),
			("city", "IMBE") => "4310330".to_string(),
			_ => value
		};

		if field.starts_with("date") || field.ends_with("date") {
			value = parse_date(&value);
		}

		if field.starts_with("value") || field.ends_with("value") || field.starts_with("quantity") || field.ends_with("quantity") {
			if value.is_empty() {
				Ok(Value::Null)
			} else {
				let res  = parse_number(&value);

				if res.is_err() {
					eprintln!("[nfe_import.parse_html.get_value] Wrong text content in in get_value({field}, {value}) !")
				}

				Ok(json!(res?))
			}
		} else {
			match (table, field) {
				("request_product", "unity") => Ok(parse_reg_ex_match(table, field, r#"KG|UN|PÇ|PC"#, &value, Value::Null, Value::Null)?),
				(_, "id_import") => Ok(parse_reg_ex_match(table, field, r#"\d+"#, &value, json!(0), Value::Null)?),
				("request_payment", "type") => Ok(parse_id(table, field, &value)?),
				(_ , "type" | "proc_emi" | "fin_nfe" | "tp_imp" | "id_dest" | "ind_final" | "ind_pres" | "country" | "city" | "uf" | "ind_ie_dest" | "cfop" | "orig" | "cst_icms" | "pay_by" | "crt" | "cnae" | "mod" | "serie" | "numero" | "ncm" | "cest") => Ok(parse_id(table, field, &value)?),
				_ => Ok(json!(value)),
			}
		}
	}

    fn parse_tables(table: &str, text: &str) -> Result<Value, Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		std::fs::write(format!("/tmp/parse_tables-{table}.html"), text.as_bytes())?;
		let regex = regex::Regex::new(r#"<table class=""#)?;//.case_insensitive(true).build()?;
		let mut tables: VecDeque<&str> = regex.split(text).collect();
        let mut list = vec![];

		if let Some(_) = tables.pop_front() {
			if let Some(table_labels) = tables.pop_front() {
				let regex = regex::Regex::new(r#"<label>(?P<label>.*)<\/label>"#)?;//.case_insensitive(true).build()?;
				let mut names = vec![];

				for capture in regex.captures_iter(table_labels) {
					if let Some(label) = capture.name("label") {
						names.push(label_to_name(label.as_str()));
					} else {
						eprintln!("[nfe_import.parse_html_parse_cobranca] Missing label !");
					}
				}

				if let Some(table_values) = tables.pop_front() {
					let regex = regex::Regex::new(r#"<span>(?P<value>.*)<\/span>"#)?;//.case_insensitive(true).build()?;
					let mut values = vec![];

					for capture in regex.captures_iter(table_values) {
						if let Some(value) = capture.name("value") {
							values.push(value.as_str());
						} else {
							eprintln!("[nfe_import.parse_html_parse_cobranca] Missing value !");
						}
					}

					if names.len() == values.len() {
						let mut obj = json!({});

						for (index, field) in names.iter().enumerate() {
							obj[field] = json!(get_value(table, field, values[index])?);
						}

						list.push(obj);
					}
				}
			}
		}

        Ok(json!(list))
    }

    fn parse_fields(table :&str, text: &str) -> Result<Value, Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		std::fs::write(format!("/tmp/parse_fields-{table}.html"), text.as_bytes())?;
		let mut obj = json!({});
		let regex = regex::Regex::new(r#"<label(\s+.*)?>(?P<label1>.*)</label><span(\s+.*)?>(?P<value1>.*)</span>|<td class="(?P<label2>.*)"><span>(?P<value2>.*)</span>"#)?;//.case_insensitive(true).build()?;

        for capture in regex.captures_iter(text) {
			let label = if let Some(matc) = capture.name("label1") {
				matc.as_str()
			} else {
				if let Some(matc) = capture.name("label2") {
					matc.as_str()
				} else {
					eprintln!("[nfe_import.parse_html_parse_fields] Missing label !");
					continue;
				}
			};

			let field = label_to_name(label);

			let str = if let Some(matc) = capture.name("value1") {
				matc.as_str()
			} else {
				if let Some(matc) = capture.name("value2") {
					matc.as_str()
				} else {
					eprintln!("[nfe_import.parse_html_parse_fields] Missing value !");
					continue;
				}
			};

			if str.is_empty() {
				#[cfg(debug_assertions)]
				println!("{table}.{field} is empty !");
				continue;
			}

			let value = get_value(table, &field, str)?;

			if let Some(old_value) = obj.get(&field) {
                if old_value != &value {
					match (table, field.as_str()) {
						("informacoes_adicionais", "descricao") => {},
						_ => {
							let msg = format!("[NfeParser.parseHtml.parseFields] : TODO alread maped field {field}, oldValue={old_value}, newValue={value}");
							eprintln!("{msg}");
							return Err(msg)?;
						}
					}
				}
			}

			//std::fs::write(format!("/tmp/tmp-{field}.txt"), value.to_string().as_bytes())?;
			obj[field] = json!(value);
        }

        Ok(obj)
    }

    fn parse_inputs(obj :&mut Value, table :&str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		std::fs::write(format!("/tmp/parse_inputs-{table}.html"), text.as_bytes())?;
		let regex = regex::Regex::new(r#"name="(?P<name>\w+)"\s*value="(?P<value>.*)""#)?;

        for capture in regex.captures_iter(text) {
			let label = if let Some(matc) = capture.name("name") {
				matc.as_str().to_case(convert_case::Case::Snake)
			} else {
				eprintln!("[nfe_import.parse_html_parse_fields] Missing label !!!!");
				continue;
			};

			let str = if let Some(matc) = capture.name("value") {
				matc.as_str()
			} else {
				eprintln!("[nfe_import.parse_html_parse_fields] Missing value !!!!");
				continue;
			};

			let field = label_to_name(&label);

			if str.is_empty() {
				#[cfg(debug_assertions)]
				println!("{table}.{field} is empty !");
				continue;
			}

			let value = get_value(table, &field, str)?;

			if let Some(old_value) = obj.get(&field) {
                if old_value != &value {
					let msg = format!("[NfeParser.parseHtml.parseFields] : TODO alread maped field {field}, oldValue={old_value}, newValue={value}");
					eprintln!("{msg}");
					return Err(msg)?;
				}
			}

			//std::fs::write(format!("/tmp/tmp-{field}.txt"), value.to_string().as_bytes())?;
			obj[field] = json!(value);
        }

        Ok(())
    }

	fn parse_field_sets(table :&str, text: &str) -> Result<Value, Box<dyn std::error::Error>> {
		#[cfg(debug_assertions)]
		std::fs::write(format!("/tmp/parse_field_sets-{table}.html"), text.as_bytes())?;
		let regex = regex::Regex::new(r#"<fieldset><legend\s*(class="toggle")?\s*>\s*"#)?;
		let mut recs: Vec<&str> = regex.split(text).collect();
		let root = recs.remove(0);
		let mut obj_out = parse_fields(table, root)?;

		for text in recs {
			let Some(pos_end) = text.find("</legend>") else {
				eprintln!("[request_product] Broken missing fieldser legend !");
				continue;
			};

			let name = label_to_name(&text[0..pos_end]);
			let name = name.as_str();

			match name {
				"situacao_atual_autorizada_ambiente_autorizacao_producao" => obj_out[name] = parse_tables(name, text)?,
				_ => obj_out[name] = parse_fields(name, text)?,
			}
		}

		Ok(obj_out)
	}

	//#[cfg(debug_assertions)]
	//tokio::fs::write("/tmp/tmp.html", html.as_bytes()).await?;
    let regex = regex::Regex::new(r#"<fieldset><legend class="titulo-aba">\s*"#)?;
    let mut recs: Vec<&str> = regex.split(&html).collect();
    let form = recs.remove(0);
    let mut obj_out = json!({});

    for html in recs {
        let Some(pos_end) = html.find("</legend>") else {
            eprintln!("Broken missing fieldser legend !");
            continue;
        };

        let name = label_to_name(&html[0..pos_end]);
		let table = name.as_str();

		match table {
			"request_nfe" => {
				let mut obj = parse_field_sets(table, html)?;
				parse_inputs(&mut obj, table, form)?;
				obj_out[table] = obj;
			},
			"request_product" => {
				let mut list = vec![];
				let regex = regex::Regex::new(r#"<table class="toggle box">"#)?;
				let mut recs: Vec<&str> = regex.split(&html).collect();
				recs.remove(0);

				for html in recs {
					let obj = parse_field_sets(table, html)?;
                    list.push(obj);
				}

				obj_out[table] = json!(list);
			},
			"request_payment" => obj_out[table] = parse_tables(table, html)?,
			_ => obj_out[table] = parse_fields(table, html)?,
		}
    }

	#[cfg(debug_assertions)]
    tokio::fs::write(format!("/tmp/tmp.json"), obj_out.to_string().as_bytes()).await?;
    Ok(obj_out)
}

async fn process_broker_message(message :&Value) -> Result<(), Box<dyn std::error::Error>> {
    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

	let Some(file_path) = message.get("file") else {
		Err(r#"Missing field "file" in broker message !"#)?
	};

	let Some(file_path) = file_path.as_str() else {
		Err(r#"Field "file" in broker message is not string !"#)?
	};

    let contents = {
        let mut file = match File::open(file_path).await {
			Ok(file) => file,
			Err(err) => {
				eprintln!("[nfe_import.process_broker_message] File::open({file_path}) : {err}");
				return Err(err)?;
			}
		};

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        bytes
    };
	// verifica e processa site da sefaz RS
    if let Some(_pos_ini) = find_subsequence(&contents, b"Content/css/SiteSefaz.css") {
        if let Some(pos_ini) = find_subsequence(&contents, b"<html>") {
            let contents = &contents[pos_ini..];

            if let Some(pos_end) = find_subsequence(&contents, b"</html>") {
                let text = std::str::from_utf8(&contents[..pos_end+7])?.to_string();
                let obj = parse_html(&text).await?;
				merge(file_path, message, &obj).await?;
            }
        }
    }
    // TODO : verificar e processar xml
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "kafka")]
    {
		let query = std::env::var("CONNECT_BOOTSTRAP_SERVERS").unwrap_or("127.0.0.1:9092".to_owned());
		let re = regex::Regex::new(r"(?P<host>[\w\.\-]{1,64}):(?P<port>\d{1,5})").unwrap();
		let cap = re.captures(&query).unwrap();
		let host = cap.name("host").unwrap().as_str().to_string();
		let port: u16 = cap.name("port").unwrap().as_str().parse().unwrap();
		let bootstrap_addrs = vec![samsa::prelude::BrokerAddress {host, port}];

		let partitions = vec![0];
		let topic_name = "nfe".to_string();
		let assignment = samsa::prelude::TopicPartitionsBuilder::new().assign(topic_name, partitions).build();

		let consumer = samsa::prelude::ConsumerBuilder::<samsa::prelude::TcpConnection>::new(bootstrap_addrs,assignment).await.unwrap().build();

		use futures::{StreamExt};
		let stream = consumer.into_stream();
		// have to pin streams before iterating
		tokio::pin!(stream);

		// Stream will do nothing unless consumed.
		//while let Some(Ok(part)) = parts.next().await {
		while let Some(Ok(batch)) = stream.next().await {
			for it in batch {
				let bytes = it.value;
				let message = std::str::from_utf8(&bytes);

				match message {
					Ok(message) => {
						let value = serde_json::from_str::<Value>(message);

						match value {
								Ok(value) => {
									if let Err(err) = process_broker_message(&value).await {
										eprintln!("[nfe_import.main] Error of process_broker_message({message}) : {}", err);
									}
								},
								Err(err) => eprintln!("[nfe_import.main] Kafaka value is not valid json ({message}) : {}", err),
							}
					},
					Err(err) => eprintln!("[nfe_import.main] Kafaka value is not utf8 string : {}", err),
				}
			}
		}
    }
    #[cfg(not(feature = "kafka"))]
	{
        let host = std::env::var("REDIS_HOST").unwrap_or("127.0.0.1".to_owned());
        let client = redis::Client::open(format!("redis://{host}/")).map_err(|err| format!("Redis failt : {err}"))?;
		let mut con = client.get_connection().expect("conn");

		const GROUP_NAME: &str = "nfe_group";
		let res: Result<(), _> = redis::Commands::xgroup_create_mkstream(&mut con, "nfe", GROUP_NAME, "$");

		if let Err(e) = res {
			println!("[nfe_import.main] Group already exists: {e:?}")
		}

		let opts = redis::streams::StreamReadOptions::default().block(10000).group(GROUP_NAME, "nfe-import-1");

		loop {
			let srr: Result<redis::streams::StreamReadReply, redis::RedisError> = redis::Commands::xread_options(&mut con,
				&["nfe"],
				&[">"],
				&opts);

			let srr = match srr {
				Ok(srr) => srr,
				Err(err) => {
					eprintln!("[nfe_import.main] {err}");
					break;
				},
			};

			for redis::streams::StreamKey { key, ids } in srr.keys {
				println!("[nfe_import.main] Stream key {key}");
				let mut list_id = vec![];

				for redis::streams::StreamId { id, map } in ids {
					println!("[nfe_import.main] Redis Events Stream : ID {id}");

					for (n, s) in map {
						println!("[nfe_import.main] Redis Events Stream : Stream n {n}");

						if let redis::Value::BulkString(bytes) = s {
							let message = std::str::from_utf8(&bytes);

							match message {
								Ok(message) => {
									println!("[nfe_import.main] Redis Events Stream : {message}.");
									let value = serde_json::from_str::<Value>(message);

									match value {
											Ok(value) => {
												if let Err(err) = process_broker_message(&value).await {
													eprintln!("[nfe_import.main] Redis Events Stream : Error of process_broker_message({message}) : {}", err);
												}
											},
											Err(err) => eprintln!("[nfe_import.main] Redis Events Stream : value is not valid json ({message}) : {}", err),
										}
								},
								Err(err) => eprintln!("[nfe_import.main] Redis Events Stream : value is not utf8 string : {}", err),
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
    use serde_json::json;
    use crate::process_broker_message;

    #[tokio::test]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
		let list = [
			//"43251004796820000273650010000241461902414669" // Não consegue tratar o campo troco !
			//"43251008422432000100650020000041211008937715" // Não consegue tratar o campo troco ! (Na verdade é compra de outra pessoa que foi tirado cpf na nota por engano)
			"43251120958548000156651020002478621102553576"
		];

		let mut server_connection = rufs_base_rust::client::ServerConnection::new("http://localhost:8080/nfe/");
		let customer_user = "12345678901.guest";
		let password = "e10adc3949ba59abbe56e057f20f883e";
		server_connection.login("/login", customer_user, password).await?;
		let mut message = json!({});
		message["authorization"] = json!(server_connection.login_response.jwt_header);

		for item in list {
			message["file"] = json!(format!("data/80803792034-juliana-{item}.html"));
			process_broker_message(&message).await?;
		}

        Ok(())
    }
}
