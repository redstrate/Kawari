function spawnZoneEObjs(zone, eobjs)
    for idx, data in pairs(eobjs) do
        local eobj <const> = {
            index                       = idx,
            kind                        = data[1],
            flag                        = data[2],
            base_id                     = data[3],
            entity_id                   = data[4],
            layout_id                   = data[5],
            content_id                  = data[6],
            owner_id                    = data[7],
            bind_layout_id              = data[8],
            scale                       = data[9],
            shared_group_timeline_state = data[10],
            rotation                    = data[11],
            fate                        = data[12],
            permission_invisibility     = data[13],
            args1                       = data[14],
            args2                       = data[15],
            args3                       = data[16],
            unk1                        = data[17],
            position                    = data[18]
        }

        zone:spawn_eobj(eobj)
    end
end
