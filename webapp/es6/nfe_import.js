const regExpChaveAcesso = /(?<chaveNfe>\b\d{44,47}\b)/;
let stoped = true;

function load_frame(url) {
	const regExpChaveAcessoResult = regExpChaveAcesso.exec(url);
	const chaveNfe = regExpChaveAcessoResult.groups.chaveNfe;
	// TODO : carregar página da sefaz em um frame
	// https://dfe-portal.svrs.rs.gov.br/Dfe/QrCodeNFce?p=43250693332468000326653010001148641285816550|2|1|1|07351969fd9a99aac273d67a62839c6e69297426
	// https://nfce.set.rn.gov.br/portalDFE/NFCe/DadosNFCe.aspx?tipoConsulta=completo
	// https://www.sefaz.rs.gov.br/dfe/Consultas/ConsultaPublicaDfe?chaveNFe=43250693332468000326653010001148641285816550
	const nfce = document.getElementById("nfe_import-nfce");
	nfce.src = url;
	const nfce_completo = document.getElementById("nfe_import-nfce_completo");
	nfce_completo.src = `https://www.sefaz.rs.gov.br/dfe/Consultas/ConsultaPublicaDfe?chaveNFe=${chaveNfe}`;
	const input_file = document.getElementById("nfe_import-input_file");
	input_file.hidden = false;
}

function init(debug) {
	const listDevices = [];
	listDevices.push(`<option value="file">Importar arquivo</option>`);
	let inputFileHidden = "hidden";

	if (debug == true) {
		inputFileHidden = "";
	}
	// QRCODE reader Copyright 2011 Lazar Laszlo, http://www.webqr.com
	window.qrcode.canvas_qr2 = document.createElement('canvas');
	window.qrcode.canvas_qr2.id = "qr-canvas";
	window.qrcode.qrcontext2 = window.qrcode.canvas_qr2.getContext('2d');
	//		window.qrcode.canvas_qr2.width = video.videoWidth;
	//		window.qrcode.canvas_qr2.height = video.videoHeight;

	window.qrcode.callback = (response) => {
		if (response.startsWith("error") == false) {
			console.log(`[RequestController.qrcode] :`, response);
/*
			let chaveNFe;

			if (response.includes("?chNFe=") == true) {
				chaveNFe = response.substr(response.indexOf("?chNFe=") + 7, 44);
			} else if (response.includes("?p=") == true) {
				chaveNFe = response.substr(response.indexOf("?p=") + 3, 44);
			}

			load_frame(chaveNFe);
*/
			load_frame(response);
		}
	}

	if (navigator.mediaDevices != undefined) {
		navigator.mediaDevices.getUserMedia({video: true, audio: false}).then(stream => {
			if (navigator.mediaDevices.enumerateDevices != undefined) {
				navigator.mediaDevices.enumerateDevices().then(devices => {
					let deviceId = null;

					devices.forEach(device => {
						if (device.kind === "videoinput") {
							if (deviceId == null) {
								deviceId = device.deviceId;
							}

							listDevices.push(`<option value="${device.deviceId}">${device.label}</option>`);
							console.log(device);
							console.log(navigator.mediaDevices.getSupportedConstraints());
							console.log(device.toJSON());
							console.log(device.getCapabilities());
						}
					});

					const element = document.getElementById("nfe_import-video_input");
					element.innerHTML = listDevices.join("");
				});
			} else {
				console.error("enumerateDevices() not supported.");
			}
		});
	} else {
		console.error("mediaDevices() not supported.");
	}

	const html = `
<h4>Importar NFe consumidor - SEFAZ-RS</h4>

<form name="nfe_import" class="form-horizontal" role="form" data-rufs-module="nfe_import.js">
	<div class="form-group">
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

    <input id="nfe_import-input_file_qr_code" type="file" accept="image/*" ${inputFileHidden}></input>
    <input id="nfe_import-input_file" type="file"></input>
</form>

<video id="nfe_import-qr_video" hidden></video>

<canvas id="nfe_import-out_canvas" width="320" height="240" hidden></canvas>

<pre id="nfe_import-result"></pre>

<iframe id="nfe_import-nfce" src="" title="NFCE" width="800" height="600"></iframe>
<iframe id="nfe_import-nfce_completo" src="" title="NFCE Complete" width="800" height="600"></iframe>
`;
	return html;
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
}

function play(stream) {
	const input_file = document.getElementById("nfe_import-input_file");
	input_file.hidden = true;
	const video = document.getElementById("nfe_import-qr_video");
	video.hidden = false;
	video.srcObject = stream;

	let processImage = () => {
		if (document.getElementById("nfe_import-qr_video") != undefined) {
			window.qrcode.qrcontext2.drawImage(video, 0, 0);
			console.log(video.videoWidth, video.videoHeight);

			try {
				window.qrcode.decode();
				stop();
			} catch (e) {
				console.log(e);

				if (stoped == false) {
					setTimeout(processImage, 1000);
				}
			}
		} else {
			stop();
		}
	}

	video.play().then( () => {
		stoped = false;
		processImage();
	});
}

function deviceChange(videoInput) {
	stop();

	if (videoInput == "file") {
		return;
	}

	let options = {
		deviceId: {exact: videoInput},
		//facingMode: 'environment',
		frameRate: {ideal: 2},
		width: {ideal: 1280}
	};

	navigator.mediaDevices.getUserMedia({video: options, audio: false}).then(stream => {
		play(stream);
	});
}

async function process(params) {
	//{form_id: target, event: "OnClick", data: {}}
	//{form_id: target, event: "OnChange", data}
	const {loginResponse, form_id, event, data} = params;
	const urlParams = new URLSearchParams(document.location.search);
	const debug = urlParams.get("debug") == "true";
	const viewResponse = {
		html: {},
		changes: {},
		tables: {},
		aggregates: {},
		forms: {}
	};

	if (event == "OnClick") {
		if (form_id.includes("#!/app/")) {
			const formId = "nfe_import";
			const html = init(debug);
			viewResponse.html[formId] = html;
		} else if (form_id == "nfe_import-exit") {
			stop();
		}
	} else if (event == "OnChange") {
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
							console.log(`[RequestController.deviceChange.File] : Quagga:`, result.codeResult.code);
							load_frame(result.codeResult.code);
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

					if (text.includes("quoted-printable")) {
						const list = ["=\n", "=\r\n", "=3D", "=20", "=C3=A1", "=C3=A7", "=C3=B5", "=C3=A3", "=C3=A9", "=C3=B3", "=C3=BA", "=C3=AD", "=C3=A0", "=C3=AA"];
						const replaces = ["", "", "=", " ", "á", "ç", "õ", "ã", "é", "ó", "ú", "í", "à", "ê"];

						for (let i = 0; i < list.length; i++) {
							text = text.replaceAll(list[i], replaces[i]);
						}
					}

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

					text = text
						.replaceAll("\n", "")
						.replaceAll(/\s{2,}/g, " ")
						.replaceAll("> <", "><")
						.replaceAll(" style=\"display:none\"", "")
						.replaceAll("; display: none", "")
						.replaceAll("\" /", "\"/");
					const match = regExpChaveAcesso.exec(text);

					if (match != null && match.groups.chaveNfe != null) {
						const formData = new FormData();
						formData.append('file', text); // 'file' is the name the server expects
						const headers = {"Authorization": "Bearer " + loginResponse.jwt_header};
						fetch(`upload?id=${match.groups.chaveNfe}`, {method: "POST", headers, body: formData})
						.then(response => response.json())
						.then(data => {
							console.log('Upload successful:', data);
						})
						.catch(error => {
							console.error('Upload failed:', error);
						});
					} else {
						alert(`O arquivo ${file} não contém a informação da chave/ID de Nota Fiscal Eletrônica !`);
					}
				};

				reader.onerror = (e) => {
					console.error("Error reading file:", e.target.error);
				};

				reader.readAsText(file);
			}
		} else if (form_id == "nfe_import-video_input") {
			deviceChange(data[form_id]);
		}
	}

	return viewResponse;
}

export {process};
