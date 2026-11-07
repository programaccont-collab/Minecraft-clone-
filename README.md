# Blocks v0.4 (original Rust voxel game for Android)

Moving panorama main menu, infinite world, 5 biomes (Plains, Desert, Snowy, Forest, Lush),
caves, water, lava, lush caves (moss + pools), dungeon spawners, creative + survival,
inventory with crafting, Blob mobs, sounds, chat + commands.

Controls: left stick = move, drag right side = look, UP = jump/swim up, DN = fly down,
HIT = attack/break, PUT = place/eat, hotbar = select. Top right: INV, FLY, CHAT, II.
Desktop: WASD, Space, Shift, mouse drag, E hit, Q put, 1-8, T chat, / command, I inventory.

Survival: hit trees for wood -> INV -> craft planks, sticks, sword, pickaxe (needed for stone).
Blobs hunt you; kill them for gel. 8 gel + 4 stone crafts a spawner. Lava burns, falls hurt.

Commands: /help /seed /tp /fly /time /give <block> [n] /gamemode s|c /heal /fov /render
/spawn /kill /speed /mobs /pos /biome /clear

## Get the APK
1. Push this folder to a GitHub repo (include the hidden .github folder).
2. Actions tab -> "Build APK" -> download artifact `blocks-apk`.
Local: docker run --rm -v "$PWD":/root/src -w /root/src notfl3/cargo-apk cargo quad-apk build
