"use strict";

const device = document.getElementById("device");
const width = document.getElementById("width");
const height = document.getElementById("height");
const handler = throttle(onInput);
device.addEventListener("input", handler);
width.addEventListener("input", handler);
height.addEventListener("input", handler);

for (const btn of document.querySelectorAll("#resolution > button"))
{
	btn.addEventListener("click", e =>
	{
		width.value = e.target.dataset.width;
		height.value = e.target.dataset.height;
		handler();
	});
}

const canvas = document.getElementById("preview");
const ctx = canvas.getContext("2d");

const take_reference_image = document.getElementById("take_reference_image");
const reference = document.getElementById("reference");
const referenceCtx = reference.getContext("2d");
take_reference_image.addEventListener("click", () => getReferenceImage(true));

let imageData;
let tl;
let tr;
let bl;
let br;

let draggingCorner = null;
canvas.addEventListener("mousedown", e =>
{
	let closestDistance = Infinity;
	let closestPoint = null;
	for (const point of [tl, tr, bl, br])
	{
		const dx = e.offsetX - point.x * canvas.width;
		const dy = e.offsetY - point.y * canvas.height;
		const distance = dx * dx + dy * dy;
		if (distance < closestDistance)
		{
			closestDistance = distance;
			closestPoint = point;
		}
	}
	if (closestDistance < 10000)
		draggingCorner = closestPoint;
});

canvas.addEventListener("mouseup", e =>
{
	draggingCorner = null;
});

canvas.addEventListener("mousemove", e =>
{
	if (draggingCorner)
	{
		draggingCorner.x = e.offsetX / canvas.width;
		draggingCorner.y = e.offsetY / canvas.height;
		render();
		handler();
	}
});

load();

async function load()
{
	await loadCameras();
	await loadConfig();
	await getReferenceImage();

	const ws = new WebSocket(`ws://${location.host}/ws/camera`);
	ws.addEventListener("message", onMessage);
}

async function loadCameras()
{
	const request = new Request("/api/cameras");
	const response = await fetch(request);
	const cameras = await response.json();
	for (const item of cameras)
	{
		const option = document.createElement("option");
		option.textContent = item;
		device.appendChild(option);
	}
}

async function loadConfig()
{
	const request = new Request("/api/config/camera");
	const response = await fetch(request);
	const config = await response.json();
	device.value = config.device;
	width.value = config.width;
	height.value = config.height;
	tl = config.top_left;
	tr = config.top_right;
	bl = config.bottom_left;
	br = config.bottom_right;
}

async function onInput()
{
	const config =
	{
		device: device.value,
		width: Number(width.value),
		height: Number(height.value),
		top_left: tl,
		top_right: tr,
		bottom_left: bl,
		bottom_right: br,
	};
	const request = new Request("/api/config/camera",
	{
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(config),
	});
	const response = await fetch(request);
	if (!response.ok)
		alert(await response.text());
}

async function onMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	imageData = toImageData(buffer);
	canvas.width = imageData.width;
	canvas.height = imageData.height;
	render();
}

function render()
{
	if (!imageData)
		return;

	ctx.putImageData(imageData, 0, 0);

	ctx.strokeStyle = "#ffffff";
	ctx.beginPath();
	ctx.moveTo(tl.x * canvas.width, tl.y * canvas.height);
	ctx.lineTo(tr.x * canvas.width, tr.y * canvas.height);
	ctx.lineTo(br.x * canvas.width, br.y * canvas.height);
	ctx.lineTo(bl.x * canvas.width, bl.y * canvas.height);
	ctx.lineTo(tl.x * canvas.width, tl.y * canvas.height);
	ctx.stroke();

	ctx.fillStyle = "#00ff00";
	ctx.beginPath();
	ctx.ellipse(tl.x * canvas.width, tl.y * canvas.height, 5, 5, 0, 0, 2 * Math.PI);
	ctx.fill();

	ctx.fillStyle = "#ff0000";
	ctx.beginPath();
	ctx.ellipse(tr.x * canvas.width, tr.y * canvas.height, 5, 5, 0, 0, 2 * Math.PI);
	ctx.fill();
}

async function getReferenceImage(take = false)
{
	const queryString = new URLSearchParams({ take }).toString();
	const request = new Request("/api/config/camera/reference?" + queryString,
	{
		method: "POST",
	});
	const response = await fetch(request);
	const image = await createImageBitmap(await response.blob());
	const w = reference.width = image.width;
	const h = reference.height = image.height;
	referenceCtx.drawImage(image, 0, 0, w, h);

	for (let y = 0; y < h / STONE_SIZE; y++)
	{
		for (let x = 0; x < w / STONE_SIZE; x++)
		{
			referenceCtx.fillStyle = "black";
			referenceCtx.fillRect((x + 0.5) * STONE_SIZE - 1, (y + 0.5) * STONE_SIZE - 1, 2, 2);
		}
	}
}