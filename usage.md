# Usage

This page contains an in-depth description of how the various interface modes of the Go board work.

## Calibration Mode

This is the default mode, which is active whenever there is no client connected. It is used to align the projector display with the board.

### Display

In calibration mode, there is a small white dot rendered on every intersection of the board. There is also a larger green dot rendered in the top left corner, and a red dot in the top right corner, which are used for orientation.

### Input

There is no input available in this mode.

## Game Mode

This is the most useful mode, used for playing games.

### Display

There is a stripe on the first line of the board, on the side of the player whose turn it is to play. When the stripe is white, it is waiting for that player to place a stone on the board. When the stripe is yellow, it indicates that an incoming move has arrived from the client, and it is waiting for the user to place a stone at the indicated location on the board.

When it is waiting for the user to play an incoming move, the intersection corresponding to the move will contain a blinking white dot. If the incoming move is a pass, it will skip this step and immediately start waiting for the user's next move.

If the vision model is having trouble reading part of the board, or what it sees does not match what it expects to see, the problematic location will be highlighted with a red blinking pattern. For example this may happen if stones are off-centre from their intersections.

### Input

To make a move (either your own move or an incoming move), simply place a stone on the board, and remove captured stones if there are any.

When it is waiting for an incoming move, playing the indicated move will update the display and it will start waiting for the user's next move.

To pass, place two of your stones on the board (on any two intersections) simultaneously. The stones are not considered to be actual moves, so you must remove them again in case the game resumes.

To resign, place two of your opponent's stones on the board simultaneously.

## Game Over Mode

This mode is used to display a game result. Note that this mode is not used by all clients. For example, games played via `saigo-gtp` will not display a result.

### Display

The winner's half of the board is highlighted in green, and the loser's half is highlighted in red.

### Input

There is no input available in this mode.

## Training Mode

This mode is only used while collecting training data. It consists of randomly generated patterns, used to acclimate the vision model to a wider variety of inputs.