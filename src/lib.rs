use chrono::NaiveDateTime;
use rufs_base_rust::client::{DataView, DataViewWatch, ServerConnection, DataViewProcessAction, HtmlElementId};
#[cfg(target_arch = "wasm32")]
use rufs_base_rust::client::DataViewManagerWrapper;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(target_arch = "wasm32")]
use web_log::println;

pub struct RufsNfe {}

#[derive(Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
     id              : usize,
     #[serde(rename = "type")]
     typ            : usize,
     state           : usize,
     person           : String,
     person_dest      : String,
     date             : NaiveDateTime,
     additional_data  : Option<String>,
     products_value   : Option<f64>,
     services_value   : Option<f64>,
     transport_value  : Option<f64>,
     desc_value       : Option<f64>,
     sum_value        : Option<f64>,
     payments_value   : Option<f64>,
 }

#[derive(Debug,Deserialize,Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestProduct {
    id :Option<usize>,
    request :usize,
    product :usize,
    quantity :f64,
    #[serde(default)]
    value :f64,
    value_item :Option<f64>,
    value_desc :Option<f64>,
    value_freight :Option<f64>,
    cfop :Option<usize>,
    value_all_tax :Option<f64>,
    serials :Option<String>,   
 }

impl RufsNfe {
    
    fn request_payment_adjusts(data_view_payment : &mut DataView, watcher : &dyn DataViewWatch, server_connection: &ServerConnection, request: &Request, typ :Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        let remaining_payment = request.sum_value.unwrap_or(0.0) - request.payments_value.unwrap_or(0.0);

        if data_view_payment.filter_results.len() == 0 {
            let value = json!(remaining_payment);
            //println!("[request_payment_adjusts] : value  = {}", value);
            data_view_payment.set_value(server_connection, watcher, "value", &value, None)?;
        }

        let account = data_view_payment.params.instance.get("account").unwrap_or(&Value::Null);
        println!("[request_payment_adjusts] : old account  = {}", account);

        if account.is_null() {
            let accounts = data_view_payment.field_results.get("account").ok_or("expected list of accounts")?;

            let typ = if let Some(typ) = typ {
                typ
            } else {
                data_view_payment.params.instance.get("type").unwrap_or(&json!(1)).as_u64().unwrap_or(1)
            };
    
            if typ == 1 {
                if accounts.len() > 0 {
                    let account = accounts[accounts.len()-1].get("id").ok_or("missing field id in account")?.clone();//accounts[0].id;//
                    //println!("[request_payment_adjusts] 1 : new account  = {}", account);
                    data_view_payment.set_value(server_connection, watcher, "account", &account, None)?;
                }
            } else {
                if accounts.len() > 1 {
                    let account = accounts[accounts.len()-2].get("id").ok_or("missing field id in account")?.clone();//accounts.len()-2
                    //println!("[request_payment_adjusts] 2 : new account  = {}", account);
                    data_view_payment.set_value(server_connection, watcher, "account", &account, None)?;
                }
            }
        }

        Ok(())
    }

}

impl DataViewWatch for RufsNfe {

    fn check_set_value(&self, data_view :&mut DataView, element_id: &HtmlElementId, server_connection: &ServerConnection, field_name: &str, field_value: &Value) -> Result<bool, Box<dyn std::error::Error>> {
        if data_view.data_view_id.schema_name == "request" {
            let schema_name = &element_id.data_view_id.schema_name;

            if schema_name == "requestProduct" && ["quantity", "value", "valueDesc"].contains(&field_name) {
                if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| &item.data_view_id.schema_name == schema_name) {
                    if data_view_child.params.instance.get("product").is_none() {
                        return Ok(true);
                    }

                    if field_name != "value" && data_view_child.params.instance.get("value").is_none() {
                        // TODO : se valor unitário está ausente, pegar o valor do cadastro de produtos.
                        data_view_child.set_value(server_connection, self, "value", &json!(0.0), None)?;
                    }

                    if field_name != "quantity" && data_view_child.params.instance.get("quantity").is_none() {
                        data_view_child.set_value(server_connection, self, "quantity", &json!(1.0), None)?;
                    }

                    if field_name != "valueDesc" && data_view_child.params.instance.get("valueDesc").is_none() {
                        data_view_child.set_value(server_connection, self, "valueDesc", &json!(0.0), None)?;
                    }

                    let field_value :f64 = match field_value {
                        Value::Number(field_value) => field_value.as_f64().ok_or("expected type is f64")?,
                        _ => todo!(),
                    };

                    let mut request_product: RequestProduct = serde_json::from_value(data_view_child.params.instance.clone())?;

                    let value_item = if field_name == "quantity" {
                        let value_desc = request_product.value_desc.unwrap_or(0.0);
                        (field_value * request_product.value) - value_desc
                    } else if field_name == "value" {
                        let value_desc = request_product.value_desc.unwrap_or(0.0);
                        (request_product.quantity * field_value) - value_desc
                    } else if field_name == "valueDesc" {
                        (request_product.quantity * request_product.value) - field_value
                    } else {
                        request_product.value_item.unwrap_or(0.0)
                    };

                    data_view_child.set_value(server_connection, self, "valueItem", &json!((value_item * 100.0).trunc() / 100.0), None)?;
                    let product_value_old = f64::trunc(request_product.quantity * request_product.value * 100.0) / 100.0;
                    let product_desc_value_old = request_product.value_desc.unwrap_or(0.0);

                    match field_name {
                        "quantity" => request_product.quantity = field_value,
                        "value" => request_product.value = field_value,
                        "valueDesc" => request_product.value_desc = Some(field_value),
                        _ => todo!()
                    }

                    let product_value_new = f64::trunc(request_product.quantity * request_product.value * 100.0) / 100.0;
                    let product_desc_value_new = request_product.value_desc.unwrap_or(0.0);
                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    let products_value_old = request.products_value.unwrap_or(0.0);
                    let desc_value_old = request.desc_value.unwrap_or(0.0);
                    let request_products_value_new = products_value_old - product_value_old + product_value_new;
                    let request_desc_value_new = desc_value_old - product_desc_value_old + product_desc_value_new;
                    //let element_id_parent = &HtmlElementId::new(data_view.data_view_id.schema_name.clone(), None, None, data_view.data_view_id.action, None, None);
                    data_view.set_value(server_connection, self, "productsValue", &json!(request_products_value_new), None)?;
                    data_view.set_value(server_connection, self, "descValue", &json!(request_desc_value_new), None)?;
                    data_view.set_value(server_connection, self, "sumValue", &json!(((request_products_value_new - request_desc_value_new) * 100.0).trunc()/100.0), None)?;
                    let data_view_payment = data_view.childs.iter_mut().find(|item| item.data_view_id.schema_name == "requestPayment").ok_or_else(|| format!("Missing child {} in parent {}", "requestPayment", data_view.data_view_id.schema_name))?;
                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    RufsNfe::request_payment_adjusts(data_view_payment, self, server_connection, &request, None)?;
                }
            }

            if schema_name == "requestPayment" && ["type"].contains(&field_name) {
                if let Some(data_view_child) = data_view.childs.iter_mut().find(|item| &item.data_view_id.schema_name == schema_name) {
                    let typ = field_value.as_u64().unwrap_or(1);
                    // due_date
                    if [1,4,10,11,12,13].contains(&typ) {
                        let value = data_view.params.instance.get("date").ok_or("check_set_value 1 : context")?;
                        data_view_child.set_value(server_connection, self, "dueDate", value, None)?;
                    }
                    // payday
                    if [1,4,10,11,12,13].contains(&typ) {
                        let value = data_view.params.instance.get("date").ok_or("check_set_value 2 : context")?;
                        //data_view_child.params.instance["payday"] = value.clone();
                        data_view_child.set_value(server_connection, self, "payday", value, None)?;
                    }

                    let request: Request = serde_json::from_value(data_view.params.instance.clone())?;
                    RufsNfe::request_payment_adjusts(data_view_child, self, server_connection, &request, Some(typ))?;
                }
            }
        }

        Ok(true)
    }
     
    fn check_save(&self, _data_view :&mut DataView, element_id: &HtmlElementId, _server_connection: &ServerConnection) -> Result<(bool, DataViewProcessAction), Box<dyn std::error::Error>> {
        let action = if ["new-rufs_user", "new-request"].contains(&element_id.data_view_id.id.as_str()) {
            DataViewProcessAction::Edit
        } else {
            DataViewProcessAction::Search
        };

        Ok((true, action))
    }
     
    fn menu(&self) -> Value {
        json!({
            "Cadastros": {
                "Clientes e Fornecedores": "person/search",
                "Produtos": "product/search",
                "Serviços": "service/search",
                "Contas": "account/search",
                "Requisições": "request/search",
                "Usuários": "rufs_user/search",
            },
            "Movimento": {
                "Financeiro": "request_payment/search",
                "Estoque": "stock/search",
                "Vendas": "request_product/search",
                "Serviços": "request_service/search",
                "Consertos": "request_repair/search",
            },
            "Rotinas": {
                "Compra": "request/new?instance.type=1&instance.state=10",
                "Venda": "request/new?instance.type=2&instance.state=10",
                "Conserto": "request/new?instance.type=2&instance.state=10",
                "Importar": "nfe_import.js/?instance.type=1&instance.state=10",
            },
            "Tabelas": {
                "Pessoas": "person/search",
                "Confaz Cest": "confaz_cest/search"
            }
        })
    }

}

#[cfg(target_arch = "wasm32")]
lazy_static::lazy_static! {
    static ref WATCHER: Box<dyn DataViewWatch> = Box::new(RufsNfe{}) as Box<dyn DataViewWatch>;
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = DataViewManager)]
pub struct DataViewManagerWrapperApp {
    data_view_manager_wrapper :DataViewManagerWrapper<'static>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_class = DataViewManager)]
impl DataViewManagerWrapperApp {

    #[wasm_bindgen(constructor)]
	pub fn new(path: &str) -> Self {
        use rufs_base_rust::client::DataViewManager;
        let data_view_manager = DataViewManager::new(path, &WATCHER);
        let data_view_manager_wrapper = DataViewManagerWrapper{data_view_manager};
        Self {data_view_manager_wrapper}
    }

	pub async fn login_from_response(&mut self, params :JsValue) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.login_from_response(params).await
    }

    pub async fn process(&mut self, params :JsValue) -> Result<JsValue, JsValue> {
        self.data_view_manager_wrapper.process(params).await
    }

}
