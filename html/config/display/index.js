"use strict";

const image_width = document.getElementById("image_width");
const image_height = document.getElementById("image_height");
const angle = document.getElementById("angle");
const x = document.getElementById("x");
const y = document.getElementById("y");
const width = document.getElementById("width");
const height = document.getElementById("height");
const perspective_x = document.getElementById("perspective_x");
const perspective_y = document.getElementById("perspective_y");
const handler = throttle(onInput);
image_width.addEventListener("input", handler);
image_height.addEventListener("input", handler);
angle.addEventListener("input", handler);
x.addEventListener("input", handler);
y.addEventListener("input", handler);
width.addEventListener("input", handler);
height.addEventListener("input", handler);
perspective_x.addEventListener("input", handler);
perspective_y.addEventListener("input", handler);

for (const btn of document.querySelectorAll("#resolution > button"))
{
	btn.addEventListener("click", e =>
	{
		image_width.value = e.target.dataset.width;
		image_height.value = e.target.dataset.height;
		handler();
	});
}

loadConfig();

async function loadConfig()
{
	const request = new Request("/api/config/display");
	const response = await fetch(request);
	const config = await response.json();
	image_width.value = config.image_width;
	image_height.value = config.image_height;
	angle.value = config.angle;
	x.value = config.x;
	y.value = config.y;
	width.value = config.width;
	height.value = config.height;
	perspective_x.value = config.perspective_x;
	perspective_y.value = config.perspective_y;
}

async function onInput()
{
	const config =
	{
		image_width: Number(image_width.value),
		image_height: Number(image_height.value),
		angle: Number(angle.value),
		x: Number(x.value),
		y: Number(y.value),
		width: Number(width.value),
		height: Number(height.value),
		perspective_x: Number(perspective_x.value),
		perspective_y: Number(perspective_y.value),
	};
	const request = new Request("/api/config/display",
	{
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(config),
	});
	await fetch(request);
}
