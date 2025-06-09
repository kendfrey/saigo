# API Reference

Saigo's API has two main parts: the WebSocket API and the HTTP API. The WebSocket API is primarily used for streams of commands or events during gameplay, while the HTTP API is request/response based and is primarily used for configuration. Third-party clients will typically only use the WebSocket API.

WebSocket endpoints are prefixed with `/ws`, while HTTP endpoints are prefixed with `/api`.

Unless otherwise specified, messages are in JSON format.

## Table of Contents
- [WebSocket API](#websocket-api)
	- [`/ws/board`](#wsboard)
	- [`/ws/board-camera`](#wsboard-camera)
	- [`/ws/camera`](#wscamera)
	- [`/ws/control`](#wscontrol)
	- [`/ws/display`](#wsdisplay)
	- [`/ws/game`](#wsgame)
	- [`/ws/raw-board`](#wsraw-board)
- [HTTP API](#http-api)
	- [`/api/cameras`](#apicameras)
	- [`/api/config/board`](#apiconfigboard)
	- [`/api/config/camera`](#apiconfigcamera)
	- [`/api/config/camera/reference`](#apiconfigcamerareference)
	- [`/api/config/delete`](#apiconfigdelete)
	- [`/api/config/display`](#apiconfigdisplay)
	- [`/api/config/load`](#apiconfigload)
	- [`/api/config/profiles`](#apiconfigprofiles)
	- [`/api/config/save`](#apiconfigsave)
- [Data Types](#data-types)
	- [`PlayerMove`](#playermove)
	- [`ImageData`](#imagedata)
- [Notes](#notes)
	- [Row-Major Order](#row-major-order)

## WebSocket API

### `/ws/board`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type:
	```ts
	(" " | "B" | "W")[][]
	```

	A 2D array representing the stones and empty spaces on the board in [row-major order](#row-major-order).

	Produced when the arrangement of stones on the board changes. Always reflects the state of the physical board, even if it's not a legal game state or there is no game in progress.

### `/ws/board-camera`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type: [`ImageData`](#imagedata)

	The image captured by the camera, cropped to contain only the board.

	Produced for every frame captured by the camera.

### `/ws/camera`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type: [`ImageData`](#imagedata)

	The raw image captured by the camera, without any cropping.

	Produced for every frame captured by the camera.

### `/ws/control`

Note: Only one client can be connected to this endpoint at a time.

#### Commands

- Type:
	```ts
	{
		type: "reset",
	}
	```

	Resets Saigo to calibration mode.

- Type:
	```ts
	{
		type: "new_training_pattern",
	}
	```

	Generates a new random pattern on the display and enables training mode if it is not already enabled.

- Type:
	```ts
	{
		type: "new_game",
		user_color: "B" | "W", // The color assigned to the user
	}
	```

	Starts a new game.

- Type:
	```ts
	{
		type: "play_move",
		move: PlayerMove,
	}
	```

	[`PlayerMove`](#playermove)

	Sends an external move to be played on the board, a pass, or a resignation.
	
	Note: The standard way to indicate the result of a game is to send a resignation from the losing player (which may be the user). This applies to all results, including by score or by timeout.

#### Events

This endpoint does not produce any events.

### `/ws/display`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type: [`ImageData`](#imagedata)

	The image to be projected onto the board.

	Produced whenever the image to be displayed changes.

### `/ws/game`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type:
	```ts
	PlayerMove
	```

	[`PlayerMove`](#playermove)

	A move that was played.

	Produced when the user makes a move, passes, or resigns during a game. Also produced when the user places the opponent's move on the board.

### `/ws/raw-board`

#### Commands

This endpoint does not accept any commands.

#### Events

- Type:
	```ts
	[number, number, number, number][][]
	```

	A 2D array of quadruples containing the vision model's predictions for each intersection, in [row-major order](#row-major-order). Each intersection's prediction contains the following probabilities, in order:
	- No stone
	- Black stone
	- White stone
	- Obscured/other

	Produced for every frame captured by the camera.

## HTTP API

### `/api/cameras`

Methods: GET

Body:

```ts
string[] // The list of available camera devices
```

### `/api/config/board`

Methods: GET, PUT

Body:

```ts
{
	width: number, // The width of the board
	height: number, // The height of the board
}
```

### `/api/config/camera`

Methods: GET, PUT

Body:

```ts
{
	device: string, // The name of the camera to use
	width: number, // The horizontal resolution of the camera
	height: number, // The vertical resolution of the camera
	top_left: // The position of the top left intersection within the frame
	{
		x: number, // 0.0 = left, 1.0 = right
		y: number, // 0.0 = top, 1.0 = bottom
	},
	top_right: // The position of the top right intersection within the frame
	{
		x: number,
		y: number,
	},
	bottom_left: // The position of the bottom left intersection within the frame
	{
		x: number,
		y: number,
	},
	bottom_right: // The position of the bottom right intersection within the frame
	{
		x: number,
		y: number,
	},
}
```

### `/api/config/camera/reference`

Methods: POST

Request body: none

Response body: image/png

Query parameters:
- `take: "true" | "false"`

If `take` is true, the reference image will be captured from the camera, stored in the current configuration, and returned in the response. If false, the current reference image will be returned.

### `/api/config/delete`

Methods: POST

Body: none

Query parameters:
- `profile: string`

Deletes the specified configuration profile from disk. Does not change the current configuration.

### `/api/config/display`

Methods: GET, PUT

Body:

```ts
{
	image_width: number, // The horizontal resolution of the display image
	image_height: number, // The vertical resolution of the display image
	angle: number,
	x: number,
	y: number,
	width: number,
	height: number,
	perspective_x: number,
	perspective_y: number,
}
```

### `/api/config/load`

Methods: POST

Body: none

Query parameters:
- `profile: string`

Loads the specified configuration profile from disk into the current configuration.

### `/api/config/profiles`

Methods: GET

Body:

```ts
string[] // The list of profiles available on disk
```

### `/api/config/save`

Methods: POST

Body: none

Query parameters:
- `profile: string`

Saves the current configuration to disk with the specified name. Does not change the current configuration.

## Data Types

### `PlayerMove`

```ts
{
	type: "move",
	location: string, // The intersection played, in SGF format
	player: "B" | "W",
}
|
{
	type: "pass",
	player: "B" | "W",
}
|
{
	type: "resign",
	player: "B" | "W",
}
```

Represents a move, pass, or resignation made by a player during a game.

### `ImageData`

Image data is serialized in an uncompressed binary format, consisting of the following sections in order:

- `width`: 4 bytes
- `height`: 4 bytes
- `data`: `width * height * 4` bytes

`width` and `height` are the size of the image in pixels. `data` is an array of pixel values in [row-major order](#row-major-order). Each pixel value consists of one byte each for the red, green, blue, and alpha channels, in that order, for a total of 4 bytes per pixel.

## Notes

### Row-Major Order

When an array is in row-major order, the elements are stored in order from left to right and then top to bottom. This means that the first row is stored first, followed by the second row, and so on.

When a 2D array `array` is in row-major order, `array[0]` represents the top row, and `array[0][0]` represents the top left corner. For example, on a 19x19 Go board, `array[3][15]` is the star point in the top right corner.