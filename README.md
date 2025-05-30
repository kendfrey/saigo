# Saigo

Saigo (佐為碁) is a program which adds smart capabilities to a physical Go board using a camera and projector. The purpose of this project is to provide a cheaper alternative to expensive electronic Go boards.

## Table of Contents

- [How Does It Work?](#how-does-it-work)
- [Requirements](#requirements)
	- [Hardware Suggestions](#hardware-suggestions)
- [Getting Started](#getting-started)
- [Modes of Play](#modes-of-play)
- [Usage](#usage)
- [Contributing](#contributing)
- [API Reference](#api-reference)
- [Troubleshooting](#troubleshooting)
- [Contact](#contact)

## How Does It Work?

Saigo lets you play Go on a real life board, even when you have no one else to play with. Instead of clicking on a screen, you can place stones on the board and have the camera keep track of the moves for you. When you place a move on the board, it will send the move to your opponent. When your opponent makes a move, the projector will highlight the move on the board until you place the stone in its place.

Saigo might be for you if:
- You have a nice Go set but you don't often have anyone to play over the board with
- You prefer the atmosphere of playing on a real board instead of on a screen
- You wish the commercial smart Go boards weren't so expensive
- You're willing to spend the time to set up the hardware yourself

Saigo might **not** be for you if:
- You want something that works out of the box without any setup
- You need something 100% reliable, or you're a tournament organizer
- You want something portable

## Requirements

Saigo itself does not provide any hardware, so you will have to set that up yourself. The following items are required:
- A Go board and stones
- A computer to run Saigo on
	- A CUDA-capable GPU is recommended for performance, but not required
- A camera (webcam) connected to the computer
- A projector connected to the computer as an external display

Depending on how and where you set it up, you might also need:
- A tripod or some other equipment to mount the camera and projector above the board
- Extension cables to connect the camera and/or projector to the computer

#### Hardware Suggestions

In addition to my Go set, I'm currently using the following hardware ($67 USD):
- [Generic mini projector](https://www.amazon.ca/dp/B07T2B9YP1) ($26)
	- Tends to malfunction if the power supply is plugged into the projector before being plugged into the wall
	- Only 240p resolution, which is actually fine for this application
- [Catitru webcam](https://www.amazon.ca/dp/B0D2R24B9C) ($14)
	- Has a bit of fisheye distortion which slightly affects tracking accuracy
- [Cable Matters Active USB extension cable](https://www.amazon.ca/dp/B00KY9M51O) ($15)
- [Snowkids 15ft HDMI cable](https://www.amazon.ca/dp/B08F7W11RV) ($12)

## Getting Started

For installation and configuration instructions, see [Getting Started](getting-started.md).

## Modes of Play

For instructions on how to use the included clients for different types of game or study, see [Clients](clients.md).

## Usage

For details on how information is displayed on the board and how to interact with it, see [Usage](usage.md).

## Contributing

The easiest way to contribute is to provide data for training the image recognition model, especially if the current version is having trouble reading your board. See [Contributing Training Data](training-data.md) for instructions on how to record training data.

Code contributions (via pull request) are also welcome. Assuming you are familiar with Rust and Cargo, the only other prerequisite for building this project is to install LibTorch following the instructions at [Getting Started](getting-started.md).

## API Reference

If you would like to build your own client app (e.g. a custom game mode or an integration with an internet Go server), see the documentation at [API Reference](api.md).

## Troubleshooting

- When I start the program, I get an error something like this: `The code execution cannot proceed because c10.dll was not found. Reinstalling the program may fix this problem.`
	- Make sure you have installed LibTorch and added it to your PATH environment variable, as described in [Getting Started](getting-started.md).

- The screensaver comes on, the display turns off, or the computer goes to sleep during the game.
	- The display UI tries to prevent this as long as it's active. Click on it to make sure it's the active window.

- The camera tracking is inaccurate or not working.
	- Use the hidden http://localhost:5410/debug/ page to check the raw tracking data. Black and white stones should appear as black and white circles on the camera overlay. Red circles indicate the camera cannot read the board (for example, if a player's hand is in the way).

- The camera feed is distorted or keeps flickering.
	- If you're using a USB extension cable, try plugging the camera directly into the computer. If that solves the distortion, you may need to use a different extension cable (preferably an active USB extension cable).

- The projector displays random nonsense when I turn it on.
	- This can be caused by plugging the power cable into the projector before plugging it into the wall socket. Try plugging it into the wall socket first.

- The display UI shifts or displays in the wrong position.
	- Try increasing the projector's display resolution. If the display is below the browser's minimum window size, it may cause issues.

## Contact

If you have any questions or comments, you can probably find me in the [Computer Go Community](https://discord.gg/VF7PmAatzj) Discord server, or you can email me at the address in my GitHub profile.