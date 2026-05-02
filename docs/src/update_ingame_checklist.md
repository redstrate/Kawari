# In-game Testing Checklist
When a new patch is released and you're testing in-game, it's easy to forget all the features Kawari supports, so one should ensure the following items function as expected. For all these, ensure that the console is not reporting unknown opcodes, or an unexpected/wrong opcode (e.g. a party member kick opcode when you're trying to invite someone to a party).

## Login/logout
- [ ] Enter the lobby.
- [ ] Create/delete characters.
- [ ] Enter the world.
- [ ] Ensure the server greeting message is displayed, along with the current number of online players.
- [ ] Log out.
- [ ] Ensure the console reports the client disconnecting cleanly.

## World
- [ ] In the Ruby Sea, ensure that diving, underwater portals into Tamamizu/Sui-no-Sato and surfacing all work.
- [ ] Cross a zone exit line to ensure that changing areas works.
- [ ] In the Gold Saucer, ensure that the water jet lifts/elevators function by lifting you up to the top floor.
- [ ] In Solution Nine, use one of the teleport pads to ensure they send you to the correct destination area.
- [ ] Moving around the world should uncover the fog of war on the map. The console should report when the client discovers new sub-areas of zones.

## Actions
- [ ] Ensure that ActionRequests and ActionResults work: try teleporting to another area and using an attack on an enemy. These should also be visible to other players.

## Movement
- [ ] Ensure the console doesn't report unknown states when moving, jumping or pivoting (default controls: A & D on Standard controls).
- [ ] Mount, ensure it completes, and then test your movement speed vs running.
- [ ] Check that you can ride as a passenger/pillion with a second player's mount. Riding pillion should also be visible to other players.
- [ ] While mounted, ensure crossing a zone line and teleporting keep you mounted. Note that riding pillion will not keep you mounted currently.

## Inventory
- [ ] Ensure the server sends your client the correct inventory, currency, and so on.
- [ ] Ensure inventory operations work: move an item to an empty slot and check that the console output reports neither an unknown operation nor the incorrect operation (e.g. Exchange instead of Move or CombineStack).
- [ ] Check that shops work: buy an item, sell it back, and then buy it back, ensuring that gil is subtracted or added appropriately, and your item is returned to you after buyback. Log messages appropriate to buying, selling, and buybacks should also be displayed.
- [ ] Ensure weapon and hat toggles work: their visibility should change for both you and nearby players.
- [ ] Ensure equipping gear works: changing gear should be visible for both you and nearby players.
- [ ] Ensure consumable visual effect (vfx) items like the DAM or any fireworks function as intended.

## Debug and GM Commands
- [ ] Run the `!inspect` command to ensure debug commands can be invoked.
- [ ] (Optional, but recommended) Check many or all of the debug commands to ensure they function as described in debug_commands.md.
GM commands are trickier as there are at least three different "kinds", so to ensure all known kinds are tested, here are simple examples to use:
- [ ] GMCommand `//gm teri_info`: ensure that the command prints information about the current zone to the chat window.
- [ ] GMCommandName: `//gm pos <x> <y> <z>`: ensure you are moved elsewhere in the current area.
- [ ] GMCommandName2: `//gm buddy_name <name>` (also requires you to target yourself or another player): check the console output for a test message in response to you invoking this command.

## Social
- [ ] Ensure zone chat (Say, Shout, Yell, custom Emote) functions: a second client should be able to see your name and messages.
- [ ] Ensure regular emotes (not /em chat mode) display correctly to other players.

Test party functionality:
- [ ] Invite a player to a party: the party HUD should populate correctly, and the party social list menu should show both players with the correct party status (leader, member), zone information, and so on.
- [ ] Promote a player to leader: ensure that they're correctly set as party leader.
- [ ] Perform a ready check: ensure that the votes match each player's responses (the vote starter will automatically be yes).
- [ ] Perform a countdown: ensure that the countdown appears on each player's screen.
- [ ] Share a strategy board: ensure all party members receive the strategy board and that it looks correct.
- [ ] Share a strategy board in real-time: ensure all party members receive the invitation and can see the updates happening live.
- [ ] After sharing boards, ensure there is no error message in-game saying things are already being shared.
- [ ] Send a message from both players to ensure party chat is functional.
- [ ] Mark a target with a sign (party menu -> Target Signs). These should be visible to everyone in the party if they're in the same zone.
- [ ] Place a waymarker (party menu -> Waymarks), and ideally, a waymarker preset if you're in a duty finder instance. These should be visible to everyone in the party if they're in the same zone. Clearing all waymarks should also remove them all in the same area.
- [ ] Kick a party member: the console should report the target was kicked, not that they left or disbanded.
- [ ] Disband the party: the console should report the party was disbanded, not that anyone left or was kicked.

Test the friend list:
- [ ] Send another player a friend request and decline it on the receiving end: ensure there are no "ghost" entries leftover on either player's list after declining.
- [ ] Send another request and accept this time: it should show correctly in both lists and both characters' floating names should appear in the clients' friend colours.
- [ ] Open the friend list and ensure that it shows both players online with correct zone information, classjob level, and so on.
- [ ] Remove that friend: ensure both players are removed from each other's lists and that their floating names return to their original colours.

Test cross-world linkshells:
- [ ] Create a cross-world linkshell: ensure the CWLS UI refreshes properly with the newly created shell.
- [ ] Invite a player to it: the CWLS UI should reflect on the inviter's end that the new member is an invitee and is in the member list. On the invitee's end they should be able to see all members and that they are an invitee.
- [ ] Decline the invite: it should display on both ends that the invitee declined the invite, and the CWLS UI should no longer show them as being in the linkshell.
- [ ] Accept the invite: the CWLS UI should show on both ends that the invite was accepted and that the invitee is now a regular member.
- [ ] Kick the member: the CWLS UI should no longer show them in the list, and on both ends it should show that they were kicked.
- [ ] Promote the member to Leader and Master: the CWLS UI should show on both ends that the promotions worked.
- [ ] Send a message from both players to ensure linkshell chat is functional.
- [ ] Disband the linkshell: the CWLS UI should not show that linkshell anymore, and all players should see a message that they have left the shell.

Test Moogle Mail:
- [ ] Send a letter to a friend, with attachments and a max-length message, ideally in Japanese, Chinese, or another language that uses multi-byte Unicode glyphs.
- [ ] Receive a letter from a friend.
- [ ] The delivery moogle should correctly tell you how many letters you have waiting.
- [ ] The server status bar (top right corner of the screen by default) should show the correct number of pending new letters.
- [ ] Take attachments from the letter(s) and ensure they enter your inventory correctly.
- [ ] Delete the letter(s).

## Duty Finder
- [ ] Use `!unlockcontent 4` or `!unlock all` to unlock Satasha or various duty finder contents. Log out and back in if necessary.
- [ ] Queue for Sastasha using the Duty Finder UI.
- [ ] Enter the duty by accepting the prompt.
- [ ] The intro cutscene should start as normal.
- [ ] Once the duty begins, play through it as normal or use debug/GM commands to speed things up if desired.
- [ ] Also test that Abandon Duty works.

## Housing
- [ ] Enter a housing ward and ensure the zone loads correctly.
- [ ] Enter a housing interior and ensure the zone loads correctly.
- [ ] In an apartment, give yourself at least one furniture item.
- [ ] Test moving the furniture from your inventory directly to the storeroom and back: there should be no ghost items or lost/deleted items.
The following tests should also be visible to others in the zone upon completion of the actions.
- [ ] Enter the furniture placement UI and place the furniture down in the world.
- [ ] Use the move function and move that item to another location.
- [ ] Use the rotate function and spin it around.
- [ ] Remove the furniture back to the player's inventory or to the storeroom.
- [ ] Move an item to the storeroom, then place it down in the world again. Prior to placing it, a 3D preview should appear that faces towards your character.

## Events
- [ ] Enter any inn and log out. Log back in, and ensure the "waking up" cutscene plays.
- [ ] While in the inn, interact with the Unending Journey and ensure any of its cutscenes play.
- [ ] While in the inn, interact with the bed, ensure dreamfitting works and doesn't crash the client while finishing.
- [ ] Ensure the inn room can be exited via the door.

## Misc/Unimplemented tasks
- [ ] Ensure the opening for all three city-states works. Play through their initial quests until the server sends you to the "real" versions of their areas.
- [ ] Set a title for your character. These should be visible to other players.
- [ ] Visit the market board and ensure you can view items.
- [ ] Open the party finder and ensure you see the test entries.
For the following, ensure the following items display their correct unimplemented messages when trying to do them.
- [ ] Fellowships & Fellowship Finder
- [ ] Trading
- [ ] Set a friend group/marker on a friend in the friend list
