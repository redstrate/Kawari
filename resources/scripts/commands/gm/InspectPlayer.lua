required_rank = GM_RANK_DEBUG
command_sender = "[inspect] "

function getItemCondition(condition)
    return (condition / 30000) * 100
end

function onCommand(args, player)
    info = "\z
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
    main_hand = player.inventory.equipped.main_hand
    off_hand = player.inventory.equipped.off_hand
    head = player.inventory.equipped.head
    body = player.inventory.equipped.body
    hands = player.inventory.equipped.hands
    legs = player.inventory.equipped.legs
    feet = player.inventory.equipped.feet
    ears = player.inventory.equipped.ears
    neck = player.inventory.equipped.neck
    wrists = player.inventory.equipped.wrists
    rring = player.inventory.equipped.right_ring
    lring = player.inventory.equipped.left_ring
    scrystal = player.inventory.equipped.soul_crystal

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
               printf(player, "slot %s: (id: %s, condition: %s%%)", slot_num, slot.id, getItemCondition(slot.condition))
            end
        end
    end
end
