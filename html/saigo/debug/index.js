"use strict";

const cameraCanvas = document.getElementById("camera");
const cameraCtx = cameraCanvas.getContext("2d");

const boardCanvas = document.getElementById("board");
const boardCtx = boardCanvas.getContext("2d");

const imageWs = new WebSocket(`ws://${location.host}/ws/board-camera`);
imageWs.addEventListener("message", onImageMessage);

const rawBoardWs = new WebSocket(`ws://${location.host}/ws/raw-board`);
rawBoardWs.addEventListener("message", onRawBoardMessage);

const boardWs = new WebSocket(`ws://${location.host}/ws/board`);
boardWs.addEventListener("message", onBoardMessage);

let imageBitmap = null;
let data = null;

async function onImageMessage(event)
{
	const buffer = await event.data.arrayBuffer();
	imageBitmap = await createImageBitmap(toImageData(buffer));
	renderCamera();
}

async function onRawBoardMessage(event)
{
	data = JSON.parse(event.data);
	renderCamera();
}

async function onBoardMessage(event)
{
	renderBoard(JSON.parse(event.data));
}

function renderCamera()
{
	if (imageBitmap === null || data === null)
		return;

	cameraCanvas.width = imageBitmap.width * 3;
	cameraCanvas.height = imageBitmap.height * 3;
	cameraCtx.drawImage(imageBitmap, 0, 0, imageBitmap.width * 3, imageBitmap.height * 3);

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
			let cx = (x + 0.5) * STONE_SIZE * 3;
			let cy = (y + 0.5) * STONE_SIZE * 3;
			let r = STONE_SIZE * 1.2;
			cameraCtx.fillStyle = "white";
			cameraCtx.beginPath();
			cameraCtx.moveTo(cx, cy);
			cameraCtx.arc(cx, cy, r, angle4, angle2);
			cameraCtx.closePath();
			cameraCtx.fill();
			cameraCtx.fillStyle = "red";
			cameraCtx.beginPath();
			cameraCtx.moveTo(cx, cy);
			cameraCtx.arc(cx, cy, r, angle2, angle1);
			cameraCtx.closePath();
			cameraCtx.fill();
			cameraCtx.fillStyle = "black";
			cameraCtx.beginPath();
			cameraCtx.moveTo(cx, cy);
			cameraCtx.arc(cx, cy, r, angle1, angle3);
			cameraCtx.closePath();
			cameraCtx.fill();
		}
	}
}

function renderBoard(board)
{
	const scale = 47;
	const offset = scale * 0.5;
	const w = board[0].length;
	const h = board.length;
	boardCanvas.width = w * scale;
	boardCanvas.height = h * scale;

	boardCtx.fillStyle = "#e6ba73";
	boardCtx.fillRect(0, 0, w * scale, h * scale);
	
	boardCtx.strokeStyle = "black";
	for (let y = 0; y < h; y++)
	{
		boardCtx.beginPath();
		boardCtx.moveTo(offset, y * scale + offset);
		boardCtx.lineTo((w - 1) * scale + offset, y * scale + offset);
		boardCtx.stroke();
	}
	for (let x = 0; x < w; x++)
	{
		boardCtx.beginPath();
		boardCtx.moveTo(x * scale + offset, offset);
		boardCtx.lineTo(x * scale + offset, (h - 1) * scale + offset);
		boardCtx.stroke();
	}

	for (let y = 0; y < board.length; y++)
	{
		const row = board[y];
		for (let x = 0; x < row.length; x++)
		{
			switch (row[x])
			{
				case "B":
					boardCtx.fillStyle = "black";
					break;
				case "W":
					boardCtx.fillStyle = "white";
					break;
				default:
					continue;
			}
			boardCtx.beginPath();
			boardCtx.ellipse(x * scale + offset, y * scale + offset, scale * 0.5, scale * 0.5, 0, 0, 2 * Math.PI);
			boardCtx.fill();
		}
	}
}