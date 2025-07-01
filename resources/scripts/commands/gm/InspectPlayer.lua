required_rank = GM_RANK_DEBUG
command_sender = "[inspect] "

function getItemCondition(condition)
    return (condition / 30000) * 100
end

function onCommand(args, player)
    local info = "\z
        --- Info for player ---\n\z
        Current region: %s\n\z
        Current zone: %s (%s, %s)\n\z
        Position: %.3f %.3f %.3f\n\z
        Rotation: %.3f\n\z
        --- Currency ---\n\z
        Gil: %s\n\z
        --- Equipped items ---\n\z
        Main hand: (id: %s, condition: %s%%)\n\z
        Off hand: (id: %s, condition: %s%%)\n\z
        Head: (id: %s, condition: %s%%)\n\z
        Body: (id: %s, condition: %s%%)\n\z
        Hands: (id: %s, condition: %s%%)\n\z
        Legs: (id: %s, condition: %s%%)\n\z
        Feet: (id: %s, condition: %s%%)\n\z
        Ears: (id: %s, condition: %s%%)\n\z
        Neck: (id: %s, condition: %s%%)\n\z
        Wrists: (id: %s, condition: %s%%)\n\z
        Right Ring: (id: %s, condition: %s%%)\n\z
        Left Ring: (id: %s, condition: %s%%)\n\z
        Soul Crystal: (id: %s, condition: %s%%)\z
        "
    -- Skipping belts because they don't exist anymore.
    local main_hand = player.inventory.equipped.main_hand
    local off_hand = player.inventory.equipped.off_hand
    local head = player.inventory.equipped.head
    local body = player.inventory.equipped.body
    local hands = player.inventory.equipped.hands
    local legs = player.inventory.equipped.legs
    local feet = player.inventory.equipped.feet
    local ears = player.inventory.equipped.ears
    local neck = player.inventory.equipped.neck
    local wrists = player.inventory.equipped.wrists
    local rring = player.inventory.equipped.right_ring
    local lring = player.inventory.equipped.left_ring
    local scrystal = player.inventory.equipped.soul_crystal

    printf(player, info,
           player.zone.region_name, player.zone.place_name, player.zone.internal_name, player.zone.id,
           player.position.x, player.position.y, player.position.z,
           player.rotation, player.gil,
           main_hand.id, getItemCondition(main_hand.condition),
           off_hand.id,  getItemCondition(off_hand.condition),
           head.id,      getItemCondition(head.condition),
           body.id,      getItemCondition(body.condition),
           hands.id,     getItemCondition(hands.condition),
           legs.id,      getItemCondition(legs.condition),
           feet.id,      getItemCondition(feet.condition),
           ears.id,      getItemCondition(ears.condition),
           neck.id,      getItemCondition(neck.condition),
           wrists.id,    getItemCondition(wrists.condition),
           lring.id,     getItemCondition(lring.condition),
           rring.id,     getItemCondition(rring.condition),
           scrystal.id,  getItemCondition(scrystal.condition)
           )

    local NO_ITEM <const> = 0

    command_sender = "" -- hush further sender printfs, it looks ugly here
    printf(player, "--- Player's inventory ---")

    for page_num, page in pairs(player.inventory.pages) do
        printf(player, "--- Page %s ---", page_num)
        for slot_num, slot in pairs(page.slots) do
            if slot.id ~= NO_ITEM then
               printf(player, "slot %s: (id: %s, condition: %s%%, quantity: %s)",
                    slot_num, slot.id, getItemCondition(slot.condition), slot.quantity)
            end
        end
    end
end
