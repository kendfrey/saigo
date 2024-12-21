"use strict";

const STONE_SIZE = 16;

const canvas = document.querySelector("canvas");
const ctx = canvas.getContext("2d");

document.getElementById("first").addEventListener("click", goFirst);
document.getElementById("previous").addEventListener("click", goPrevious);
document.getElementById("next").addEventListener("click", goNext);
document.getElementById("last").addEventListener("click", goLast);

canvas.addEventListener("pointerdown", mouseDown);
canvas.addEventListener("pointermove", mouseMove);
canvas.addEventListener("pointerup", mouseUp);
canvas.addEventListener("contextmenu", e => e.preventDefault());
document.addEventListener("doubleclick", e => e.preventDefault());

let imageBitmap = null;
let currentBoard = null;
let currentLabels = null;
let currentPointer = "B";

let current = "0";
let last = "?";
load();

async function load()
{
	const response = await fetch("/api/last");
	last = await response.text();
	await goFirst();
}

async function goFirst()
{
	await go("/api/first");
}

async function goNext()
{
	await go("/api/next?" + getQueryString());
}

async function goPrevious()
{
	await go("/api/previous?" + getQueryString());
}

async function goLast()
{
	await go("/api/last");
}

async function go(url)
{
	const response = await fetch(url);
	if (!response.ok)
		return;
	current = await response.text();
	await loadCurrent();
}

async function loadCurrent()
{
	document.getElementById("progress").textContent = current + " / " + last;

	const imageRequest = fetch("/api/image?" + getQueryString());
	const labelsRequest = fetch("/api/labels?" + getQueryString());

	const imageResponse = await imageRequest;
	imageBitmap = await createImageBitmap(await imageResponse.blob());

	if (currentBoard === null)
		currentBoard = Array(imageBitmap.width * imageBitmap.height / (STONE_SIZE * STONE_SIZE)).fill(" ");

	const labelsResponse = await labelsRequest;
	if (labelsResponse.ok)
	{
		currentLabels = (await labelsResponse.text()).split("");
		for (let i = 0; i < currentLabels.length; i++)
		{
			if (currentLabels[i] !== "X")
				currentBoard[i] = currentLabels[i];
		}
	}
	else
	{
		currentLabels = structuredClone(currentBoard);
		updateLabels();
	}

	render();
}

function getQueryString()
{
	return new URLSearchParams({ index: current }).toString();
}

let isDragging = null;
function mouseDown(event)
{
	event.preventDefault();
	canvas.setPointerCapture(event.pointerId);
	const isRightClick = event.button === 2;
	const x = Math.floor(event.offsetX * 0.5 / STONE_SIZE);
	const y = Math.floor(event.offsetY * 0.5 / STONE_SIZE);
	const index = x + y * imageBitmap.width / STONE_SIZE;
	if (isRightClick)
	{
		if (currentLabels[index] === "X")
		{
			currentLabels[index] = currentBoard[index];
			isDragging = " ";
		}
		else
		{
			currentLabels[index] = "X";
			isDragging = "X";
		}
	}
	else
	{
		if (currentLabels[index] === " ")
		{
			currentLabels[index] = currentPointer;
			currentBoard[index] = currentPointer;
			currentPointer = currentPointer === "B" ? "W" : "B";
		}
		else if (currentLabels[index] === "X")
		{
			currentLabels[index] = currentBoard[index];
		}
		else
		{
			currentLabels[index] = " ";
			currentBoard[index] = " ";
		}
	}

	updateLabels();
}

function mouseMove(event)
{
	if (isDragging === null)
		return;

	const x = Math.floor(event.offsetX * 0.5 / STONE_SIZE);
	const y = Math.floor(event.offsetY * 0.5 / STONE_SIZE);
	const w = imageBitmap.width / STONE_SIZE;
	const h = imageBitmap.height / STONE_SIZE;
	if (x < 0 || x >= w || y < 0 || y >= h)
		return;
	const index = x + y * imageBitmap.width / STONE_SIZE;
	
	if (isDragging === " ")
		currentLabels[index] = currentBoard[index];
	else if (isDragging === "X")
		currentLabels[index] = "X";

	updateLabels();
}

function mouseUp()
{
	isDragging = null;
}

async function updateLabels()
{
	render();

	await fetch("/api/labels?" + getQueryString(),
	{
		method: "PUT",
		headers: { "Content-Type": "text/plain" },
		body: currentLabels.join(""),
	});
}

function render()
{
	canvas.width = imageBitmap.width * 2;
	canvas.height = imageBitmap.height * 2;

	ctx.drawImage(imageBitmap, 0, 0, imageBitmap.width * 2, imageBitmap.height * 2);

	let w = imageBitmap.width / STONE_SIZE;
	let h = imageBitmap.height / STONE_SIZE;
	for (let y = 0; y < h; y++)
	{
		for (let x = 0; x < w; x++)
		{
			const index = x + y * w;
			switch (currentLabels[index])
			{
				case "B":
					ctx.fillStyle = "black";
					ctx.strokeStyle = "white";
					ctx.beginPath();
					ctx.ellipse((x + 0.5) * STONE_SIZE * 2, (y + 0.5) * STONE_SIZE * 2, 6, 6, 0, 0, 2 * Math.PI);
					ctx.fill();
					ctx.stroke();
					break;
				case "W":
					ctx.fillStyle = "white";
					ctx.strokeStyle = "black";
					ctx.beginPath();
					ctx.ellipse((x + 0.5) * STONE_SIZE * 2, (y + 0.5) * STONE_SIZE * 2, 6, 6, 0, 0, 2 * Math.PI);
					ctx.fill();
					ctx.stroke();
					break;
				case "X":
					ctx.fillStyle = "#ff1f007f";
					ctx.beginPath();
					let points =
					[
						[0.15, 0.05],
						[0.5, 0.4],
						[0.85, 0.05],
						[0.95, 0.15],
						[0.6, 0.5],
						[0.95, 0.85],
						[0.85, 0.95],
						[0.5, 0.6],
						[0.15, 0.95],
						[0.05, 0.85],
						[0.4, 0.5],
						[0.05, 0.15],
					];
					for (const [x2, y2] of points)
					{
						ctx.lineTo((x + x2) * STONE_SIZE * 2, (y + y2) * STONE_SIZE * 2);
					}
					ctx.fill();
					break;
			}
		}
	}
}