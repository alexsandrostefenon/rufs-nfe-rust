let dataViewManager;
let aggregateChartOptions = {};
let aggregateChart = {};

function data_view_show(form_id) {
	const div_id = `div-${form_id}`;
	let data_view = document.getElementById(div_id);

	if (data_view != null) {
		if (data_view.hidden == true) {
		data_view.hidden = false;
		}
	} else {
		console.error(`Missing data_view ${div_id}`);
	}
}

function updateChanges(event, changes) {
	if (changes == null) {
		return;
	}
	
	if (changes instanceof Map) {
		changes = Object.fromEntries(changes);
	}

	console.log(changes);

	for (let [formId, fields] of Object.entries(changes)) {
		if (fields instanceof Map) {
			fields = Object.fromEntries(fields);
		}

		const instanceFormId = `${formId}`;
		const form = document.getElementById(instanceFormId);
		const divFormId = `div-${formId}`;
		const divForm = document.getElementById(divFormId);

		if (form == null) {
			console.error(`Missing form ${instanceFormId}`);
			continue;
		}

		if (divForm == null) {
			console.error(`Missing divForm ${divFormId}`);
			continue;
		}

		data_view_show(formId);
		
		for (let [fieldName, value] of Object.entries(fields)) {
			if (form.hidden == true) {
				form.hidden = false;
			}

			if (divForm.hidden == true) {
				divForm.hidden = false;
			}

			if (Array.isArray(value)) {
				console.error(`Unexpected array fild (${fieldName})`);
			} else if (typeof value === 'object') {
				console.error(`Unexpected array fild (${fieldName})`);
			} else {
				const element = form.elements[fieldName];
				
				if (element == null) {
					console.log(`Missing element ${fieldName} in form ${form.name} !`);
					continue;
				}
						
				element.value = value;

				for (let flagIndex = 0; flagIndex < 64; flagIndex++) {
					const elementFlag = form[`${fieldName}-${flagIndex}`];
					
					if (elementFlag == null) {
						break;
					}

					let flagValue = (value & (1 << flagIndex)) != 0;

					if (event.target != elementFlag) {
						if (fieldName == "mask" && flagIndex== 3 && flagValue == true)
							console.log(`${fieldName}-${flagIndex}.checked = ${flagValue};`);

						elementFlag.checked = flagValue;
					} else {
						setTimeout(() => elementFlag.checked = flagValue, 100);
					}
				}
			}
		}
	}
}

function updateTables(event, tables) {
	if (tables == null) {
		return;
	}
	
	if (tables instanceof Map) {
		tables = Object.fromEntries(tables);
	}

	for (let [table_id, html] of Object.entries(tables)) {
		data_view_show(table_id);
		const div = document.getElementById(`div-table-${table_id}`);

		if (div == null) {
			console.error(`Missing table ${table_id}`);
			continue;
		}

		div.innerHTML = html;
		
		if (div.hidden == true) {
			div.hidden = false;
		}

		const div_form_id = `div-${table_id}`;
		const divForm = document.getElementById(div_form_id);
		
		if (divForm != null) {
			if (divForm.hidden == true) {
			divForm.hidden = false;
			}
		} else {
			console.error(`Missing divForm ${div_form_id}`);
		}
	}
}

var appOnChange = event => {
    let element = event.target;

	if (element.value == null) {
		console.error(`appOnChange with null value in field ${element.id}`);
		return;
	}

	if (element.type == "number" && element.value.length == 0) {
		console.error(`appOnChange with empty number value in field ${element.id}`);
		return;
	}

	let value = element.value;

	if (element.type == "checkbox") {
		value = element.checked.toString();
	}

	const form = element.form;

	if (form != null) {
		form.inert = true;
	}

	console.log(`appOnChange : ${element.id} =`, value);
	let data = {};
	data[element.id] = value;
	dataViewManager.process({form_id: element.id, event: "OnChange", data}).
	then(viewResponse => {
		if (viewResponse instanceof Map) {
			viewResponse = Object.fromEntries(viewResponse);
		}

		updateChanges(event, viewResponse.changes);
		updateTables(event, viewResponse.tables);
	}).catch(err => {
		console.error(err);
		document.querySelector('#http-working').hidden = true;
		document.querySelector('#http-error').innerHTML = err;
		document.querySelector('#http-error').hidden = false;
	}).then(() => {
		if (form != null) {
			form.inert = false;
		}
	});
}

var appOnClick = event => {
    let element = event.target;

	if (["i"].includes(element.localName)) {
		element = element.parentElement;
	}

	if (["button"].includes(element.localName) == false && element.href == null) {
		return;
	}

	if (["button", "a"].includes(element.localName) && element.dataset.bsToggle != null) {
		return;
	}

	if (element.type == "text") {
		return;
	}
	
	//event.stopPropagation();
	event.preventDefault();
	let target = element.id;

	if (target == "" && element.href != null && element.href.includes("#!")) {
		target = element.href;
	}

	if (target != null && target.length > 0 && target.startsWith("menu-") == false) {
		console.log("appOnClick : ", target);
		document.querySelector('#http-error').hidden = true;
		document.querySelector('#http-working').innerHTML = "Processando...";
		document.querySelector('#http-working').hidden = false;
		dataViewManager.process({form_id: target, event: "OnClick", data: {}}).
		then(viewResponse => {
			if (viewResponse instanceof Map) {
				viewResponse = Object.fromEntries(viewResponse);
			}

			console.log(viewResponse);
			let html_map = viewResponse.html;

			if (html_map instanceof Map) {
				html_map = Object.fromEntries(html_map);
			}

			for (let [form_id, html] of Object.entries(html_map)) {
				const div_id = `data_view_root-${form_id}`;
				let dataView = document.getElementById(div_id);

				if (dataView != null) {
					dataView.remove();
					dataView = null;
				}

				dataView = document.createElement("div");
				dataView.id = div_id;
				dataView.innerHTML = html;
				document.querySelector('#main').prepend(dataView);
			}

			document.querySelector('#http-working').hidden = true;
			updateChanges(event, viewResponse.changes);
			updateTables(event, viewResponse.tables);

			if (viewResponse.aggregates != null) {
				if (viewResponse.aggregates instanceof Map) {
					viewResponse.aggregates = Object.fromEntries(viewResponse.aggregates);
				}

				for (let [formId, aggregateResults] of Object.entries(viewResponse.aggregates)) {
					if (aggregateResults instanceof Map) {
						aggregateResults = Object.fromEntries(aggregateResults);
					}
			
					const id = `chart-aggregate--${formId}`;
					const chart = document.getElementById(id);
			
					if (chart == null) {
						console.error(`Missing chart ${id}`);
						continue;
					}

					const ctx = chart.getContext('2d');
					const xData = Array.from(Object.keys(aggregateResults));
					const yData = Array.from(Object.values(aggregateResults));
					
					if (aggregateChartOptions[formId] == null) {
						aggregateChartOptions[formId] = {type: 'bar', data: {labels: [], datasets: [{label: "", data: []}]}};
						aggregateChart[formId] = new Chart(ctx, aggregateChartOptions[formId]);
					}
					
					aggregateChartOptions[formId].data.labels = xData;
					aggregateChartOptions[formId].data.datasets[0].data = yData;
					aggregateChart[formId].update();
				}
			}

			if (viewResponse.views != null) {
				if (viewResponse.views instanceof Map) {
					viewResponse.views = Object.fromEntries(viewResponse.views);
				}

				for (let [form_id, views] of Object.entries(viewResponse.views)) {
					let data_view_id = `div-${form_id}`;
					let data_view = document.getElementById(data_view_id);

					if (data_view != null) {
						if (views.hidden == true) {
							if (data_view.hidden != true) {
								data_view.hidden = true;
							}
						}
	
						if (views.hidden == false) {
							if (data_view.hidden != false) {
								data_view.hidden = false;
							}
						}
					} else {
						console.error(`Missing data_view ${data_view_id}`);
					}
				}
			}

			if (viewResponse.forms != null) {
				if (viewResponse.forms instanceof Map) {
					viewResponse.forms = Object.fromEntries(viewResponse.forms);
				}

				for (let [form_id, form_state] of Object.entries(viewResponse.forms)) {
					if (form_state instanceof Map) {
						form_state = Object.fromEntries(form_state);
					}

					let fieldset_id = `fieldset-${form_id}`;
					let fieldset = document.getElementById(fieldset_id);

					if (fieldset != null) {
						if (form_state.disabled == false && fieldset.disabled != false) {
							fieldset.disabled = false;
						}
	
						if (form_state.disabled == true && fieldset.disabled != true) {
							fieldset.disabled = true;
						}
						
						if (form_state.hidden == true && fieldset.hidden != true) {
							fieldset.hidden = true;
						}
	
						if (form_state.hidden == false && fieldset.hidden != false) {
							fieldset.hidden = false;
						}
					} else {
						console.error(`Missing fieldset ${fieldset_id}`);
					}

					let form = document.getElementById(form_id);
					
					if (form != null) {
						if (form_state.hidden == true && form.hidden != true) {
							form.hidden = true;
						}
	
						if (form_state.hidden == false && form.hidden != false) {
							form.hidden = false;
						}
					} else {
						console.error(`Missing form ${form_id}`);
					}
				}
			}
		}).catch(err => {
			console.error(err);
			document.querySelector('#http-working').hidden = true;
			document.querySelector('#http-error').innerHTML = err;
			document.querySelector('#http-error').hidden = false;
		});
	}
	
}

var login = event => {
	document.querySelector('#http-error').hidden = true;
	document.querySelector('#http-working').innerHTML = "Aguardando resposta do servidor...";
	document.querySelector('#http-working').hidden = false;
    let element = event.target; let rowIndex = 0;
	const form = element.form;

	if (form.reportValidity()) {
		//event.stopPropagation();
		event.preventDefault();
		dataViewManager.login({user: form.user.value, password: form.password.value}).
		then(loginResponse => {
			const regExMenuSearch = /\.(new|view|edit|search)(\?)?/;
			const regExMenuReplace = "/$1$2";
			
			const addToParent = (menu, list) => {
				if (menu instanceof Map) {
					menu = Object.fromEntries(menu);
				}

				for (let [name, field] of Object.entries(menu)) {
					if (typeof field === 'object') {
						list.push(
						`<li class='nav-item dropdown'><a class='nav-link dropdown-toggle' href='#' role='button' data-bs-toggle="dropdown" aria-expanded='false' id="menu-${name}">${name}</a><ul class='dropdown-menu'>`);
						addToParent(field, list);
						list.push(`</ul>\n</li>`);
					} else {
						let schema = field.replaceAll("/", ".").replace(regExMenuSearch, regExMenuReplace);
						list.push(`<li><a class='dropdown-item' href='#!/app/${schema}'>${name}</a></li>\n`);
					}
				}
			};

			if (loginResponse instanceof Map) {
				loginResponse = Object.fromEntries(loginResponse);
			}

			let list = [];
			list.push(`<ul class='nav nav-pills'>`);
			addToParent(loginResponse.menu, list);
			list.push(`</ul>`);
			const str = list.join("\n");
			let div = document.createElement("div");
			div.innerHTML = str;
			div.addEventListener('click', appOnClick);
			document.querySelector('#menu').appendChild(div);
			form.hidden = true;
			document.querySelector('#http-working').hidden = true;
			let schema = loginResponse.path.replaceAll("/", ".").replace(regExMenuSearch, regExMenuReplace);

			for (let element of document.querySelectorAll(`a[href='#!/app/${schema}']`)) {
				element.click();
			}
		}).catch(err => {
			console.error(err);
			document.querySelector('#http-working').hidden = true;
			document.querySelector('#http-error').innerHTML = err;
			document.querySelector('#http-error').hidden = false;
		});
	}
}

class DataViewManagerWs {

	constructor(server_url) {
		this.server_url = server_url;
    }

	login(obj_in) {
		let url = `${this.server_url}/wasm_ws/login`;
		let options = {method: "POST", body: JSON.stringify(obj_in), headers: {}};
		options.headers["Content-Type"] = "application/json";
		return fetch(url, options).
		then(response => {
			if (response.status != 200) {
				return response.text().
				then(text => {
					throw text
				});
			}
			
			return response.json();
		}).
		then(loginResponse => {
			this.token = loginResponse.jwt_header;
			return loginResponse;
		});
    }

    process(obj_in) {
		let options = {method: "POST", body: JSON.stringify(obj_in), headers: {}};
		options.headers["Content-Type"] = "application/json";

		if (this.token != undefined) {
			options.headers["Authorization"] = "Bearer " + this.token;
		}

		let url = `${this.server_url}/wasm_ws/process`;
		return fetch(url, options).
		then(response => {
			if (response.status != 200) {
				return response.text().
				then(text => {
					throw text
				});
			}
			
			return response.json();
		});
    }

}

async function run() {
	let server_url = window.location.origin + window.location.pathname;
	
	if (server_url.endsWith("/")) {
		server_url = server_url.substring(0, server_url.length - 1);
	}

	console.log(server_url);
	const urlParams = new URLSearchParams(document.location.search);

	if (urlParams.get("mode") == "ws") {
		dataViewManager = new DataViewManagerWs(server_url);
	} else {
		const wasm_js = await import("../../pkg/rufs_nfe_rust.js");
		await wasm_js.default();
		dataViewManager = new wasm_js.DataViewManager(server_url);
	}
	
	document.querySelector('#login-send').addEventListener('click', login);
	document.querySelector('#main').addEventListener('click', appOnClick);
	document.querySelector('#main').addEventListener('change', appOnChange);
}

run();
