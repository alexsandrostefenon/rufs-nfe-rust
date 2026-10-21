use std::collections::VecDeque;
use convert_case::Casing;
use quickxml_to_serde::{JsonArray, JsonType};
use regex::Regex;
use rufs_nfe_rust::import::{convert_obj, convert_obj_child, convert_obj_copy, import_document};
use serde_json::{json, Value};
use tokio::{fs::File, io::AsyncReadExt};

fn get_uf_code(uf :&str, default :usize) -> usize {
	match uf {
		"RS" => 43,
		"SP" => 35,
		"SC" => 42,
		"PR" => 41,
		"DF" => 53,
		"ES" => 32,
		"RJ" => 33,
		"MG" => 31,
		_ => default
	}
}

async fn parse_xml(xml: &str) -> Result<Value, Box<dyn std::error::Error>> {
	fn label_to_name(label :&str) -> String {
		let name = match label {
			"CNPJ" => "cnpjCpf",
			"CPF" => "cnpjCpf",
			"UF" => "uf",
			"xNome" => "name",
			"IE" => "ieRg",
			"?nome_fantasia" => "fantasy",
			"xLgr" => "address",
			"xEnder" => "address",
			"nro" => "addressNumber",
			"xCpl" => "complement",
			"xBairro" => "district",
			"CEP" => "zip",
			"cMun" => "city",
			"fone" => "phone",
			"cPais" => "country",
			"?inscricao_municipal" => "im",
			"?cnae_fiscal" => "cnae",
			"CRT" => "crt",
			"?inscricao_suframa" => "suframa",
			"?e_mail" => "email",

			"dhEmi" => "date",
			"vNF" => "sumValue",
			"idDest" => "idDest",

			"modFrete" => "payBy",
			"pesoL" => "weight",
			"qVol" => "containersCount",
			"pesoB" => "weightFinal",

			"xProd" => "name",
			"cProd" => "model",
			"NCM" => "ncm",
			"CEST" => "cest",
			"cEAN" => "barcode",
			"uCom" => "unity",
			"qCom" => "quantity",
			"vUnCom" => "value",
			"vProd" => "valueItem",
			"?codigo_produto" => "idImport",
			"?origem_mercadoria" => "orig",
			"CFOP" => "cfop",
			"vTotTrib" => "valueAllTax",

			"vPag"=> "value",

			"vFrete" => "valueFreight",
			"?value_frete" => "transportValue",

			"?tributacao_icms" => "cstIcms",
			"tPag" => "type",
			"tpNF" => "type",
			"finNFe" => "finNfe",
			"natOp" => "natOp",
			"?tipo_operacao" => "type",

			"digVal" => "digest",

			"vDesc" => "valueDesc",

			"chNFe" => "nfeId",
			"dhSaiEnt" => "dateInOut",
			"mod" => "mod",
			"nNF" => "numero",
			"tpImp" => "tpImp",
			"cDV" => "cDv",
			"indFinal" => "indFinal",
			"indPres" => "indPres",
			"procEmi" => "procEmi",
			"verProc" => "verProc",
			"indIEDest" => "indIeDest",//enderDest
			"vII" => "valueIi",
			"vIPI" => "valueIpi",
			"vPIS" => "valuePis",
			"vCOFINS" => "valueCofins",
			"vICMS" => "valueIcms",
			"?value_icms_substituicao" => "valueIcmsSt",
			_ => &label,
		};
		name.to_string()
	}

	fn copy(obj_out: &mut Value, obj_in :&Value) -> Result<(), Box<dyn std::error::Error>> {
		let map = obj_in.as_object().ok_or("obj_in is not 'map' object!")?;

		for (field_name_in, field) in map {
			let field_name_out = label_to_name(field_name_in);

			let value = match field {
				Value::Object(_obj) => continue,
				Value::Null => continue,
				Value::String(str) => {
					if str.is_empty() {
						continue;
					}

					if ["dhEmi", "dhSaiEnt"].contains(&field_name_in.as_str()) {
						json!(&str[..19])
					} else {
						field.clone()
					}
				},
				Value::Bool(_) => continue,
				Value::Number(number) => {
					if ["CNPJ", "CPF", "IE"].contains(&field_name_in.as_str()) {
						json!(number.to_string())
					} else {
						field.clone()
					}
				},
				Value::Array(_values) => continue,
			};

			obj_out[field_name_out] = value;
		}

		Ok(())
	}

	fn adjust_person_uf(person :&mut Value, default :usize) -> usize {
		let mut uf = 43;

		if let Some(value) = person.get_mut("uf") {
			if let Some(uf_str) = value.as_str() {
				uf = get_uf_code(uf_str, default);
				*value = json!(uf);
			}
		}

		if person.get("uf").is_none() && let Some(value) = person.get("city") {
			if let Some(city) = value.as_str() && city.len() > 2 {
				uf = city[..2].parse::<usize>().unwrap_or(default);
				person["uf"] = json!(uf);
			}
		}

		if person.get("uf").is_none() {
			uf = default;
			person["uf"] = json!(uf);
		}

		uf
	}

	let config = quickxml_to_serde::Config::new_with_custom_values(false, "@", "#text", quickxml_to_serde::NullValue::EmptyObject)
						 .add_json_type_override("/nfeProc/protNFe/infProt/chNFe", JsonArray::Infer(JsonType::AlwaysString))
						 .add_json_type_override("/nfeProc/NFe/infNFe/det/prod/cEAN", JsonArray::Infer(JsonType::AlwaysString))
						 ;

	let obj = quickxml_to_serde::xml_str_to_json(xml, &config)?;

	let Some(nfe_proc) = obj.get("nfeProc") else {
		return Err(r#"Missing element "nfeProc"#)?;
	};

	let Some(prot_nfe) = nfe_proc.get("protNFe") else {
		return Err(r#"Missing element "protNFe"#)?;
	};

	let Some(inf_prot) = prot_nfe.get("infProt") else {
		return Err(r#"Missing element "infProt"#)?;
	};

	let Some(nfe) = nfe_proc.get("NFe") else {
		return Err(r#"Missing element "NFe"#)?;
	};

	let Some(inf_nfe) = nfe.get("infNFe") else {
		return Err(r#"Missing element "infNFe"#)?;
	};

	let Some(inf_adic) = inf_nfe.get("infAdic") else {
		return Err(r#"Missing element "infAdic"#)?;
	};

	let Some(total) = inf_nfe.get("total") else {
		return Err(r#"Missing element "total"#)?;
	};

	let Some(icms_tot) = total.get("ICMSTot") else {
		return Err(r#"Missing element "ICMSTot"#)?;
	};

	let Some(ide) = inf_nfe.get("ide") else {
		return Err(r#"Missing element "ide"#)?;
	};

	let Some(dest) = inf_nfe.get("dest") else {
		return Err(r#"Missing element "dest"#)?;
	};

	let mut document = json!({});
	copy(&mut document, nfe_proc)?;
	copy(&mut document, prot_nfe)?;
	copy(&mut document, inf_prot)?;
	copy(&mut document, nfe)?;
	copy(&mut document, inf_nfe)?;
	copy(&mut document, inf_adic)?;
	copy(&mut document, total)?;
	copy(&mut document, icms_tot)?;
	copy(&mut document, ide)?;
	copy(&mut document, dest)?;
	let mut uf = 43;
	// person
	{
		let Some(emit) = inf_nfe.get("emit") else {
			return Err(r#"Missing element "emit"#)?;
		};

		let Some(ender_emit) = emit.get("enderEmit") else {
			return Err(r#"Missing element "enderEmit"#)?;
		};

		let mut person = json!({});
		copy(&mut person, emit)?;
		copy(&mut person, ender_emit)?;
		uf = adjust_person_uf(&mut person, uf);
		document["person"] = person;
	}
	// person_dest
	{
		let Some(ender_dest) = dest.get("enderDest") else {
			return Err(r#"Missing element "enderDest"#)?;
		};

		let mut person = json!({});
		copy(&mut person, dest)?;
		copy(&mut person, ender_dest)?;
		uf = adjust_person_uf(&mut person, uf);
		document["personDest"] = person;
	}
	// transporte
	{
		let Some(transp) = inf_nfe.get("transp") else {
			return Err(r#"Missing element "transp""#)?;
		};

		let mut request_freight = json!({});
		copy(&mut request_freight, transp)?;

		if let Some(vol) = transp.get("vol") {
			copy(&mut request_freight, vol)?;
		}

		if let Some(transporta) = transp.get("transporta") {
			copy(&mut request_freight, transporta)?;
		}

		adjust_person_uf(&mut request_freight, uf);
		document["requestFreight"] = request_freight;
	}
	// lista de produtos
	{
		let Some(det) = inf_nfe.get("det") else {
			return Err(r#"Missing element "det""#)?;
		};

		let det = if det.is_array() {
			det.clone()
		} else {
			json!([det.clone()])
		};

		let mut list = vec![];

		for item in det.as_array().unwrap() {
			let Some(prod) = item.get("prod") else {
				return Err(r#"Missing element "prod""#)?;
			};

			let Some(imposto) = item.get("imposto") else {
				return Err(r#"Missing element "imposto""#)?;
			};

			let mut obj_out = json!({});
			copy(&mut obj_out, prod)?;
			copy(&mut obj_out, imposto)?;
			list.push(obj_out);			
		}

		document["requestProduct"] = json!(list);
	}
	// lista pagamentos
	{
		let Some(pag) = inf_nfe.get("pag") else {
			return Err(r#"Missing element "pag""#)?;
		};

		let Some(value) = pag.get("detPag") else {
			return Err(r#"Missing element "detPag""#)?;
		};

		let value = if value.is_array() {
			value.clone()
		} else {
			json!([value.clone()])
		};

		let mut list = vec![];

		for obj_in in value.as_array().unwrap() {
			let mut obj_out = json!({});
			copy(&mut obj_out, obj_in)?;
			list.push(obj_out);			
		}

		document["requestPayment"] = json!(list);
	}

	if let Some(value)  = document.get("valueItem") {
		document["productsValue"] = value.clone();
	}

	if let Some(value)  = document.get("valueDesc") {
		document["descValue"] = value.clone();
	}

	Ok(document)
}

async fn parse_html(html: &str) -> Result<Value, Box<dyn std::error::Error>> {
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
			//"informacoes_adicionais" => "request_nfe",
			//"informacoes_suplementares" => "request_nfe",
			//"icms" => "request_nfe",
			//"totais" => "request_nfe",
			"dados_emitente" => "person",
			//"emitente" => "person",
			//"destinatario" => "person_dest",
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
			"uf" => get_uf_code(&value, 43).to_string(),
			"nfe_id" => {
				let regex_nfe_id = Regex::new(r#".*(?P<id>\d{44,47}).*"#).unwrap();
				regex_nfe_id.replace_all(&value, "$id").to_string()
			}
			_ => value,
		};

		let mut value = match (field, value.as_str()) {
			("country", "BRASIL") => "1058".to_string(),
			("city", "FARROUPILHA") => "4308003".to_string(),
			("city", "GLORINHA") => "4309050".to_string(),
			("city", "IMBE") => "4310330".to_string(),
			("city", "PORTO ALEGRE") => "4314902".to_string(),
			("city", "OSASCO") => "3534401".to_string(),
			("city", "NOVA SANTA RITA") => "4313375".to_string(),
			
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
				(_, "id_import") => Ok(parse_reg_ex_match(table, field, r#"^\d{1,8}$"#, &value, json!(0), Value::Null)?),
				("request_payment", "type") => Ok(parse_id(table, field, &value)?),
				(_ , "type" | "proc_emi" | "fin_nfe" | "tp_imp" | "id_dest" | "ind_final" | "ind_pres" | "country" | "city" | "uf" | "ind_ie_dest" | "cfop" | "orig" | "cst_icms" | "pay_by" | "crt" | "cnae" | "mod" | "serie" | "numero" | "ncm" | "cest") => Ok(parse_id(table, field, &value)?),
				(_, "name") => Ok(json!(value.replace("+", " ").replace("&AMP;", " ").replace("  ", " ").trim())),
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

		let table = if name.starts_with("dados_nf_e_") {
			"request_nfe"
		} else {
			name.as_str()
		};

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

    Ok(obj_out)
}

async fn import(message :&Value) -> Result<(), Box<dyn std::error::Error>> {
    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

	let Some(file_path) = message.get("file") else {
		Err(r#"Missing field "file" in broker message !"#)?
	};

	let Some(file_path) = file_path.as_str() else {
		Err(r#"Field "file" in broker message is not string !"#)?
	};

	let Some(auth_token) = message.get("authorization") else {
		return Err(format!("Missing field authorization in message"))?;
	};

	let Some(auth_token) = auth_token.as_str() else {
		return Err(format!("Field ath is not string."))?;
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

	let regex = Regex::new(r#"(\d{8,14})"#)?;
	let customer = regex.find(file_path).ok_or("Broken customer_id")?.as_str();

    if let Some(_pos_ini) = find_subsequence(&contents, b"<nfeProc") {
		let xml = std::str::from_utf8(&contents)?;
        let mut document = parse_xml(xml).await?;
        document["customer"] = json!(customer);
		import_document(auth_token, document, true, false).await?;
	} else if let Some(_pos_ini) = find_subsequence(&contents, b"Content/css/SiteSefaz.css") {
		// processa site da sefaz RS
        if let Some(pos_ini) = find_subsequence(&contents, b"<html>") {
            let contents = &contents[pos_ini..];

            if let Some(pos_end) = find_subsequence(&contents, b"</html>") {
                let text = std::str::from_utf8(&contents[..pos_end+7])?.to_string();
                let obj = parse_html(&text).await?;
				#[cfg(debug_assertions)]
				tokio::fs::write(format!("/tmp/tmp.json"), obj.to_string().as_bytes()).await?;
				let request_nfe_snake = obj.get("request_nfe").ok_or("Missing structure request_nfe in parsed object")?;
				let mut document = convert_obj(request_nfe_snake, &[])?;
				document["person"] = convert_obj_child(&obj, "person", &[], "nfe_import.1")?;
				document["personDest"] = convert_obj_child(&obj, "person_dest", &[], "nfe_import.1")?;
				document["requestFreight"] = convert_obj_child(&obj, "request_freight", &[], "nfe_import.1")?;
				document["requestProduct"] = obj.get("request_product").ok_or("Missing field 'request_product'")?.clone();
				convert_obj_copy(&mut document, request_nfe_snake.get("emissao").ok_or("Missing 'emissao'")?, &[])?;
				convert_obj_copy(&mut document, obj.get("informacoes_adicionais").ok_or("Missing 'informacoes_adicionais'")?, &[])?;
				convert_obj_copy(&mut document, obj.get("totais").ok_or("Missing 'totais'")?, &[])?;

				if let Some(destinatario) = request_nfe_snake.get("destinatario") {
					convert_obj_copy(&mut document, destinatario, &[])?;
				}

				if let Some(date_in_out) = document.get("dateInOut") {
					if date_in_out.as_str().ok_or("broken")?.is_empty() {
						document["dateInOut"] = document.get("date").ok_or("broken")?.clone();
					}
				} else {
					document["dateInOut"] = document.get("date").ok_or("broken")?.clone();
				}

				if let Some(request_payment) = obj.get("request_payment") {
					document["requestPayment"] = request_payment.clone();
				} else {
					let date = document.get("date").ok_or("Missing field 'date' in 'request_nfe'")?.as_str().ok_or("field 'date' is not 'str'")?;

					if date.starts_with("2017") {
						let value = document.get("valueTotalNfe").ok_or("Missing field 'valueTotalNfe'.")?;
						document["requestPayment"] = json!([{"type": 90, "value": value}]);
					} else {
						return Err("Missing section 'request_payment'.")?;
					}
				}

		        document["customer"] = json!(customer);
				import_document(auth_token, document, true, true).await?;
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
									if let Err(err) = import(&value).await {
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
				println!("[nfe_import.main] V1.1.1.2606251 Stream key {key}");
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
												if let Err(err) = import(&value).await {
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
    use serde_json::{Value, json};

    #[tokio::test]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
		let list = [
  			"00000000000000000000000000000000000000000000.xml",
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
			message["file"] = json!(format!("data/12345678901-guest-{item}"));
			crate::import(&message).await?;
		}

        Ok(())
    }
}
