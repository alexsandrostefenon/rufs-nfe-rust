import { showStatus, dataViewManager, loginResponse } from "./app.js";

const regExpNfeChaveAcesso = /(?<nfeId>\b\d{22}\s?\d{22,27}\b)/;
let stoped = true;
let nfeIdOld = null;

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function log(text, data) {
	console.log(text, data);
	const element = document.getElementById("nfe_import-log");

	if (element == null) {
		return;
	}

	if (typeof text != "string") {
		text = JSON.stringify(text);
	}

	if (typeof data == "string") {
		text = text + " " + data;
	} else if (data != null) {
		text = text + " " + JSON.stringify(data);
	}

	element.value = text + "\n" + element.value;
	element.hidden = false;
}

async function upload(text) {
	const processHtml = text => {
		let pos = text.indexOf("Content-Location: https://dfe-portal.svrs.rs.gov.br/Dfe/ConsultaPublicaDfe")

		if (pos > 0) {
			text = text.substring(pos);
		}

		pos = text.indexOf("<html>");

		if (pos > 0) {
			text = text.substring(pos);
		}

		pos = text.lastIndexOf("</html>");

		if (pos > 0) {
			text = text.substring(0, pos+7);
		}

		return text
			.replaceAll("\n", "")
			.replaceAll(/\s{2,}/g, " ")
			.replaceAll("> <", "><")
			.replaceAll(" style=\"display:none\"", "")
			.replaceAll("; display: none", "")
			.replaceAll("\" /", "\"/");
	}

	if (text.includes("quoted-printable")) {
		const list = ["=\n", "=\r\n", "=3D", "=20", "=C3=A1", "=C3=A7", "=C3=B5", "=C3=A3", "=C3=A9", "=C3=B3", "=C3=BA", "=C3=AD", "=C3=A0", "=C3=AA"];
		const replaces = ["", "", "=", " ", "á", "ç", "õ", "ã", "é", "ó", "ú", "í", "à", "ê"];

		for (let i = 0; i < list.length; i++) {
			text = text.replaceAll(list[i], replaces[i]);
		}
	}

	let type = "";

	if (text.includes("<html>")) {
		type = "html";
		text = processHtml(text);
	} else if (text.includes(`,"Chave de Acesso",`)) {
		type = "csv";
		// TODO : temporário até fazer o servidor processar em background e notificar o resultado via websocket.
		{
			let count = 0;
			let list = text.split("\n");
			let nfeIdCancel = "";

			for (let line of list) {
				if (line.includes(`,"Chave de Acesso",`)) {
					continue;
				}

				const regExpNfeChaveAcessoResult = regExpNfeChaveAcesso.exec(line);

				if (regExpNfeChaveAcessoResult == null) {
					log(`Broken nfeId in CSV file :`, line);
					continue;
				}

				const nfeId = regExpNfeChaveAcessoResult.groups.nfeId.replaceAll(" ", "");

				if (nfeId == nfeIdCancel) {
					log(`nfeId ${nfeId} is canceled !`);
					continue;
				}

				if (line.includes(`,"Cancelamento",`)) {
					nfeIdCancel = nfeId;
					continue;
				}

				const data = await loadFrame(nfeId);

				if (data == null || data.nfeId == null) {
					count++;
					await sleep(2000);
				}
			}

			log(`Processed ${count} of ${list.length}.`);
			return;
		}
	}

	const match = regExpNfeChaveAcesso.exec(text);

	if (match != null && match.groups.nfeId != null) {
		const formData = new FormData();
		formData.append('file', text); // 'file' is the name the server expects
		const headers = {"Authorization": "Bearer " + loginResponse.jwt_header};
		fetch(`upload?type=${type}`, {method: "POST", headers, body: formData})
		.then(response => response.json())
		.then(data => {
			log('Upload successful: ', data);
		})
		.catch(error => {
			console.error('Upload failed:', error);
		});
	} else {
		alert(`O arquivo ${file} não contém a informação da chave/ID de Nota Fiscal Eletrônica !`);
	}
}

async function loadFrame(url) {
	const regExpNfeChaveAcessoResult = regExpNfeChaveAcesso.exec(url);

	if (regExpNfeChaveAcessoResult == null) {
		log(`Broken nfeId :`, url);
		return null;
	}

	const nfeId = regExpNfeChaveAcessoResult.groups.nfeId.replaceAll(" ", "");

	if (nfeId == nfeIdOld) {
		return null;
	}

	const headers = {
		"Authorization": "Bearer " + loginResponse.jwt_header
	};

	const response = await fetch(`rest/request_nfe?filter[nfeId]=${nfeId}`, {method: "GET", headers});
	const list = await response.json();

	if (Array.isArray(list) && list.length > 0) {
		log('Alread imported: ', nfeId);
		document.getElementById("nfe_import-qr_code").value = "";
		return list[0];
	}

	showStatus(`Importando NFE ${nfeId} ...`);
	headers["Content-Type"] = "application/json";
	return fetch(`import?id=${nfeId}`, {method: "POST", headers, body: `{}`})
	.then(response => response.json())
	.then(data => {
		log('Upload successful: ', data);
		nfeIdOld = nfeId;
		return data;
	})
	.catch(err => {
		const input_file = document.getElementById("nfe_import-input_file");
		input_file.hidden = false;
		url = `https://www.sefaz.rs.gov.br/dfe/Consultas/ConsultaPublicaDfe?nfeId=${nfeId}`;
		window.open(url, "_blank");
		return null;
	})
	.finally(data => {
		document.getElementById("nfe_import-qr_code").value = "";
		return data;
	});
}

function stop() {
	const video = document.getElementById("nfe_import-qr_video");

	if (video == null) {
		return;
	}

	const stream = video.srcObject;

	if (stream == null) {
		return;
	}

	stream.getTracks().forEach(function(track) {
		track.stop();
	});

	stoped = true;
	const input_file = document.getElementById("nfe_import-input_file");
	input_file.hidden = false;
	video.hidden = true;
}

function play(stream) {
	const input_file = document.getElementById("nfe_import-input_file");
	input_file.hidden = true;
	const video = document.getElementById("nfe_import-qr_video");
	video.hidden = false;
	video.srcObject = stream;

	let processImage = () => {
		window.qrcode.qrcontext2.drawImage(video, 0, 0);

		try {
			window.qrcode.decode();
			stop();
		} catch (e) {
			log(e);

			if (stoped == false) {
				setTimeout(processImage, 500);
			}
		}
	}

	video.play().then( () => {
		stoped = false;
		const video = document.getElementById("nfe_import-qr_video");
		const w = video.videoWidth;
		const h = video.videoHeight;
		window.qrcode.canvas_qr2.width = w;
		window.qrcode.canvas_qr2.height = h;
		window.qrcode.canvas_qr2.style.width = w + "px";
		window.qrcode.canvas_qr2.style.height = h + "px";
		window.qrcode.qrcontext2.clearRect(0, 0, w, h);
		log(`${w} x ${h}`);
		processImage();
	});
}

async function init(debug) {
	async function init_camera() {
		if (navigator.mediaDevices != undefined) {
			// QRCODE reader Copyright 2011 Lazar Laszlo, http://www.webqr.com
			window.qrcode.canvas_qr2 = document.createElement('canvas');
			window.qrcode.canvas_qr2.id = "qr-canvas";
			window.qrcode.qrcontext2 = window.qrcode.canvas_qr2.getContext('2d');

			window.qrcode.callback = async (response) => {
				if (response.startsWith("error") == false) {
					log(`[RequestController.qrcode] :`, response);
					await loadFrame(response);
				}
			}

			const listDevices = [];
			listDevices.push(`<option value="file">Importar arquivo</option>`);
			const _stream = await navigator.mediaDevices.getUserMedia({video: {width: 1280, height: 720}, audio: false});

			if (navigator.mediaDevices.enumerateDevices != undefined) {
				const devices = await navigator.mediaDevices.enumerateDevices();

				devices.forEach(device => {
					if (device.kind === "videoinput") {
						if (debug == true) {
							log(device);
							log(navigator.mediaDevices.getSupportedConstraints());
						}

						listDevices.push(`<option value="${device.deviceId}">${device.label}</option>`);

						if (device.getCapabilities != null) {
							log(device.getCapabilities());
						}
					}
				});

				const element = document.getElementById("nfe_import-video_input");
				element.innerHTML = listDevices.join("");
				const div = document.getElementById("nfe_import-div_video_input");
				div.hidden = false;
			} else {
				console.error("enumerateDevices() not supported.");
			}
		} else {
			console.error("mediaDevices() not supported.");
			log("mediaDevices() not supported.");
		}

		const element = document.getElementById("nfe_import-qr_code");

		element.onclick = event => {
			const element = document.getElementById("nfe_import-log");
			element.value = "";
			element.hidden = true;
		}
	}

	setTimeout(init_camera, 100);

	let inputFileHidden = "hidden";

	if (debug == true) {
		inputFileHidden = "";
	}

	const html = `
<h4>Importar NFe consumidor - SEFAZ-RS</h4>

<form id="nfe_import" name="nfe_import" class="form-horizontal" role="form" data-rufs-module="nfe_import.js">
	<div class="form-group" id="nfe_import-div_video_input" hidden>
	    <label for="video_input" class="col-sm-2 control-label">Dispositivo de video</label>

	    <div class="col-sm-10">
			<select class="form-control" id="nfe_import-video_input" name="video_input" required></select>
	    </div>
	</div>

    <div class="form-group">
      <div class="col-sm-offset-2 col-sm-10">
		<button class="btn btn-default" id="nfe_import-exit"><span class="glyphicon glyphicon-remove"></span> Sair</button>
      </div>
    </div>

    <div class="form-group">
		<label for="nfe_import-qr_code" class="col-sm-2 control-label">Qr-code (colar)</label>
		<input id="nfe_import-qr_code" type="text"></input>
    </div>
    <div class="form-group">
		<label for="nfe_import-input_file" class="col-sm-2 control-label">Arquivo (site Sefaz) baixado</label>
		<input id="nfe_import-input_file" type="file"></input>
    </div>
    <div class="form-group" ${inputFileHidden}>
		<label for="nfe_import-input_file_qr_code" class="col-sm-2 control-label">Qr-code (Foto/Imagem)</label>
	    <input id="nfe_import-input_file_qr_code" type="file" accept="image/*"></input>
    </div>
	<textarea style="overflow: auto;width: 100%" id="nfe_import-log" rows="5" hidden></textarea>
</form>

<video id="nfe_import-qr_video" style="width: 100%" hidden></video>

<canvas id="nfe_import-out_canvas" width="320" height="240" hidden></canvas>
`;
	return html;
}

function deviceChange(videoInput) {
	stop();

	if (videoInput == "file") {
		return;
	}

	let video_options = {
		deviceId: {exact: videoInput},
		//facingMode: 'environment',
		width: 1280,
		width: 720
	};

	navigator.mediaDevices.getUserMedia({video: video_options, audio: false}).then(stream => {
		play(stream);
	});
}

async function process(params) {
	const {form_id, event, data} = params;
	const urlParams = new URLSearchParams(document.location.search);
	const debug = urlParams.get("debug") == "true";

	let viewResponse = {
		html: {},
		changes: {},
		tables: {},
		aggregates: {},
		forms: {}
	};

	if (event == "click") {
		if (form_id.includes("#!/app/")) {
			const element = document.getElementById("data_view_root-nfe_import");

			if (element != null) {
				element.hidden = false;
			} else {
				const html = await init(debug);
				const formId = "nfe_import";
				viewResponse.html[formId] = html;
				//viewResponse.workingTimeout = 5000;
			}
		} else if (form_id == "nfe_import-exit") {
			stop();
			const element = document.getElementById("data_view_root-nfe_import");
			element.hidden = true;
		}
	} else if (event == "change") {
		if (form_id == "nfe_import-input_file_qr_code") {
			for (let file of data[form_id]) {
				let reader = new FileReader();

				reader.onload = eventReader => {
					const base64data = eventReader.target.result;
					window.qrcode.decode(base64data);
					window.Quagga.decodeSingle({
						decoder: {
							readers: ["code_128_reader"]
						},
						src: base64data
					}, result => {
						if (result && result.codeResult) {
							log(`[RequestController.deviceChange.File] : Quagga:`, result.codeResult.code);
							document.getElementById("nfe_import-qr_code").value = result.codeResult.code;
						}
					}
					);
				}

				reader.readAsDataURL(file);
			}
		} else if (form_id == "nfe_import-input_file") {
			for (let file of data[form_id]) {
				const reader = new FileReader();

				reader.onload = (e) => {
					let text = e.target.result;
					upload(text);
				};

				reader.onerror = (e) => {
					console.error("Error reading file:", e.target.error);
				};

				reader.readAsText(file);
			}
		} else if (form_id == "nfe_import-video_input") {
			deviceChange(data[form_id]);
		} else if (form_id == "nfe_import-qr_code" && data[form_id] != "") {
			const element = document.getElementById(form_id);
			const res = await loadFrame(data[form_id]);

			if (res != null) {
				//viewResponse.workingTimeout = 1000;
			}
		}
	} else if (event == "keyup") {
		const oldValue = data[form_id];

		if (form_id == "nfe_import-qr_code" && oldValue != null && (oldValue.length >= 47 || (oldValue.length >= 44 && oldValue.startsWith("55") == false))) {
			const res = await loadFrame(oldValue);

			if (res != null) {
				//viewResponse.workingTimeout = 1000;
			}
		}
	}

	return viewResponse;
}

export {process};
