"use strict";

const width = document.getElementById("width");
const height = document.getElementById("height");
const handler = throttle(onInput);
width.addEventListener("input", handler);
height.addEventListener("input", handler);

for (const btn of document.querySelectorAll("#board_size > button"))
{
	btn.addEventListener("click", e =>
	{
		width.value = e.target.dataset.width;
		height.value = e.target.dataset.height;
		handler();
	});
}

loadConfig();

async function loadConfig()
{
	const request = new Request("/api/config/board");
	const response = await fetch(request);
	const config = await response.json();
	width.value = config.width;
	height.value = config.height;
}

async function onInput()
{
	const config =
	{
		width: Number(width.value),
		height: Number(height.value),
	};
	const request = new Request("/api/config/board",
	{
		method: "PUT",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(config),
	});
	await fetch(request);
}
