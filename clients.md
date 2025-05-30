# Clients

A client is a program that connects to Saigo and controls the display and game state. Only one client can be connected at a time.

## OGS Client

*Coming soon!*

## GTP Client

The GTP client is called `saigo-gtp`. It is designed to be used by a GTP controller (e.g. Sabaki) as if it were an engine. This is useful when you want to play a game against an engine running on your computer.

It requires no commandline arguments, and will automatically connect to Saigo when it starts.

The game resets on the `clear_board` command. It will start the game when it receives the next `genmove` (as black) or `play` (as white) command. `genmove` waits for the user to make a move, and then returns that move to the controller. `play` sends the specified move to Saigo as the opponent's move.

## Gather Training Data

`gather-td` is a client designed to collect training data for the image recognition model. It puts Saigo into training mode and captures images of the board. It cannot be used for playing games.