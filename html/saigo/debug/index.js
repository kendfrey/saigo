"use strict";

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

const imageWs = new WebSocket(`ws://${location.host}/ws/board-camera`);
imageWs.addEventListener("message", onImageMessage);

const ws = new WebSocket(`ws://${location.host}/ws/raw-board`);
ws.addEventListener("message", onMessage);

let imageData = null;
let data = null;

async function onImageMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	imageData = toImageData(buffer);
	render();
}

async function onMessage(event)
{
	data = JSON.parse(event.data);
	render();
}

function render()
{
	if (imageData === null || data === null)
		return;

	canvas.width = imageData.width;
	canvas.height = imageData.height;
	ctx.putImageData(imageData, 0, 0);

	for (let y = 0; y < data.length; y++)
	{
		const row = data[y];
		for (let x = 0; x < row.length; x++)
		{
			const [_, black, white, obscured] = row[x];
			const angle1 = (obscured - 0.5) * Math.PI;
			const angle2 = -Math.PI - angle1;
			const angle3 = angle1 + black * Math.PI * 2;
			const angle4 = angle2 - white * Math.PI * 2;
			let cx = (x + 0.5) * STONE_SIZE;
			let cy = (y + 0.5) * STONE_SIZE;
			let r = STONE_SIZE * 0.4;
			ctx.fillStyle = "white";
			ctx.beginPath();
			ctx.moveTo(cx, cy);
			ctx.arc(cx, cy, r, angle4, angle2);
			ctx.closePath();
			ctx.fill();
			ctx.fillStyle = "red";
			ctx.beginPath();
			ctx.moveTo(cx, cy);
			ctx.arc(cx, cy, r, angle2, angle1);
			ctx.closePath();
			ctx.fill();
			ctx.fillStyle = "black";
			ctx.beginPath();
			ctx.moveTo(cx, cy);
			ctx.arc(cx, cy, r, angle1, angle3);
			ctx.closePath();
			ctx.fill();
		}
	}
}