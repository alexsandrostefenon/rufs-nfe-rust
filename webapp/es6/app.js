import init, { DataViewManager } from '../rufs_nfe_rust.js';

let dataViewManager;

function updateChanges(event, changes) {
	if (changes == null) {
		return;
	}
	
	console.log(changes);

	for (let [formId, fields] of changes) {
		const instanceFormId = `instance-${formId}`;
		const form = document.getElementById(instanceFormId);
		const divForm = document.getElementById(`div-instance-${formId}`);

		if (form == null || divForm == null) {
			console.error(`Missing form ${instanceFormId}`);
			continue;
		}

		for (let [fieldName, value] of fields) {
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
/*
				const fields = value;
				const form_child = form[fieldName];

				for (let [fieldName, value] of fields) {
					if (typeof value === 'object') {
					} else {
						const element = form_child[fieldName];

						if (element == null) {
							throw new Error(`Missing element ${fieldName} in form ${form_child.name} !`);
						}
						
						element.value = value;
					}
				}
	*/
			} else {
				const element = form[fieldName];
				
				if (element == null) {
					throw new Error(`Missing element ${fieldName} in form ${form.name} !`);
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

	console.log(`appOnChange : ${element.id} =`, element.value);
	dataViewManager.process_edit_target(element.id, element.value).
	then(viewResponse => {
		updateChanges(event, viewResponse.changes);
	}).catch(err => {
		console.error(err);
		document.querySelector('#http-working').hidden = true;
		document.querySelector('#http-error').innerHTML = err;
		document.querySelector('#http-error').hidden = false;
	});
}

var appOnClick = event => {
    let element = event.target;
	//event.stopPropagation();
	event.preventDefault();
	let target = element.id;

	if (element.href != null && element.href.includes("#!")) {
		target = element.href;
	}

	if (target != null && target.length > 0 && target.startsWith("menu-") == false) {
		console.log("appOnClick : ", target);
		document.querySelector('#http-error').hidden = true;
		document.querySelector('#http-working').innerHTML = "Processando...";
		document.querySelector('#http-working').hidden = false;
		dataViewManager.process_click_target(target).
		then(viewResponse => {
			console.log(viewResponse);
			// DEBUG
			if (target == "instance-delete-new-request-requestProduct") {
				console.log(viewResponse);
			}

			const html = viewResponse.html;

			if (viewResponse.form_id != null && html != null && html.length > 0) {
				const div_id = `data_view-${viewResponse.form_id}`;
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

			if (viewResponse.changes != null) {
				updateChanges(event, viewResponse.changes);
			}

			if (viewResponse.tables != null) {
				for (let [formId, html] of viewResponse.tables) {
					const div = document.getElementById(`div-table-${formId}`);
			
					if (div == null) {
						console.error(`Missing table ${formId}`);
						continue;
					}
			
					div.innerHTML = html;
					
					if (div.hidden == true) {
						div.hidden = false;
					}

					const divForm = document.getElementById(`div-instance-${formId}`);
					
					if (divForm != null && divForm.hidden == true) {
						divForm.hidden = false;
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
		const path = window.location.origin;// + window.location.pathname;
		dataViewManager = new DataViewManager(path);
		const user = form.user.value;
		const password = form.password.value;
		dataViewManager.login("/login", user, password).
		then(loginResponse => {
			const addToParent = (menu, list) => {
				for (let [name, field] of menu) {
					if (typeof field === 'object') {
						list.push(
						`<li class='nav-item dropdown'><a class='nav-link dropdown-toggle' href='#' role='button' data-bs-toggle="dropdown" aria-expanded='false' id="menu-${name}">${name}</a><ul class='dropdown-menu'>`);
						addToParent(field, list);
						list.push(`</ul>\n</li>`);
					} else {
						list.push(`<li><a class='dropdown-item' href='#!/app/${field}'>${name}</a></li>\n`);
					}
				}
			};

			let list = [];
			list.push(`<ul class='nav nav-pills'>`);
			addToParent(loginResponse.get("menu"), list);
			list.push(`</ul>`);
			const str = list.join("\n");
			let div = document.createElement("div");
			div.innerHTML = str;
			div.addEventListener('click', appOnClick);
			document.querySelector('#menu').appendChild(div);
			form.hidden = true;
			document.querySelector('#http-working').hidden = true;

			for (let element of document.querySelectorAll(`a[href='#!/app/${loginResponse.get("path")}']`)) {
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

async function run() {
	await init();
	document.querySelector('#login-send').addEventListener('click', login);
	document.querySelector('#main').addEventListener('click', appOnClick);
	document.querySelector('#main').addEventListener('change', appOnChange);
}

run();
