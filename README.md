# Jigarpich: Telegram Party Game Bot
## Overview
Welcome to Jigarpich, an open-source project that enables you to enjoy a stimulating party game online via Telegram. The game is straightforward but full of engaging challenges where players form 2-member teams, taking on the roles of describer and guesser.

The describer's task is to explain a given word for the guesser, avoiding using the word itself, its synonyms, opposites, rhymes, or translations. Meanwhile, the guesser is responsible for figuring out the word based solely on the descriptions.

To keep the game dynamic and fair, players switch roles after each round of guessing. This cycle continues until the game round concludes.

A timer tracks how long each team takes to explain and guess the words. The team with the least total time emerges as the winner, making the game a fast-paced rush to be both accurate and efficient!

## Technology Stack
* [teloxide](https://github.com/teloxide/teloxide): This library simplifies interactions with the Telegram Bot API, helping with handling updates, creating commands, and managing different states of the bot.
* [dashmap](https://github.com/xacrimon/dashmap): A highly efficient concurrent hashmap that enables high-speed operations, even with multiple threads interacting with the map.
