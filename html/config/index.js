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

const save_profile = document.getElementById("save_profile");
const load_profile = document.getElementById("load_profile");

document.getElementById("save").addEventListener("click", async () =>
{
	const queryString = new URLSearchParams({ profile: save_profile.value }).toString();
	const request = new Request("/api/config/save?" + queryString,
	{
		method: "POST",
	});
	const response = await fetch(request);
	if (!response.ok)
		alert(await response.text());
	save_profile.value = "";
	await loadProfiles();
});

document.getElementById("load").addEventListener("click", async () =>
{
	const queryString = new URLSearchParams({ profile: load_profile.value }).toString();
	const request = new Request("/api/config/load?" + queryString,
	{
		method: "POST",
	});
	const response = await fetch(request);
	if (!response.ok)
		alert(await response.text());
	load_profile.value = "";
	await loadConfig();
});

load();

async function load()
{
	await loadConfig();
	await loadProfiles();
}

async function loadConfig()
{
	const request = new Request("/api/config/board");
	const response = await fetch(request);
	const config = await response.json();
	width.value = config.width;
	height.value = config.height;
}

async function loadProfiles()
{
	const request = new Request("/api/config/profiles");
	const response = await fetch(request);
	const profiles = await response.json();
	load_profile.innerHTML = "";
	for (const profile of profiles)
	{
		const option = document.createElement("option");
		option.textContent = profile;
		load_profile.appendChild(option);
	}
	load_profile.value = "";
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
	const response = await fetch(request);
	if (!response.ok)
		alert(await response.text());
}
