## Terra Websocket API

This is an API where you can see the state of terra in the `town-square` (For example the player in the zone). 

Currently, the API only respond to ping/pong check by `0x0d` and otherwise only listen to `0x0e` and `0x0f` packet 
from the schema defined by `0x0a` from initial, which then convert the `0x0e`/`0x0f` packet to the JSON object.

It used the JWT token from https://terra.hackclub.com. This is the same JWT token
used for the `Authorization` header when the browser send requests to
https://api.terra.hackclub.com, and you can obtain the JWT there (It seems to be valid for 7 days).

You can try the project by listen to the websocket at `wss://ws_terra.iau.sh/ws` which tell you the game 
state every 5s. (Or `wss://ws_terra.iau.sh/ws_high` for up to update every 200ms for higher definition data)

### Notice

The code contains significant amount of websocket logic code copied from 
[iauu/news-aggregator](https://github.com/iauu/news-aggregator), some docker setup copied from
[iauu/kasu](https://github.com/iauu/kasu) and some of the binary parsing have been assisted with AI.
