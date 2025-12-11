//! Handling all things related to the event system.

use mlua::Function;

use crate::{Event, EventFinishType, ZoneConnection, lua::LuaPlayer};
use kawari::{
    common::{EventHandlerType, ObjectTypeId},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, Condition, EventScene, EventStart, EventType,
        SceneFlags, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    /// Starts a scene for the current event.
    pub async fn event_scene(
        &mut self,
        target: &ObjectTypeId,
        event_id: u32,
        scene: u16,
        scene_flags: SceneFlags,
        params: Vec<u32>,
    ) {
        let scene = EventScene {
            actor_id: *target,
            event_id,
            scene,
            scene_flags,
            params_count: params.len() as u8,
            params,
            ..Default::default()
        };
        if let Some(ipc) = scene.package_scene() {
            self.send_ipc_self(ipc).await;
        } else {
            tracing::error!(
                "Unable to play event {event_id}, scene {:?}, scene_flags {scene_flags}!",
                scene
            );
            self.event_finish(event_id, EventFinishType::Normal).await;
        }
    }

    /// Finishes the current event, including resetting any conditions set during the start of said event.
    pub async fn event_finish(&mut self, handler_id: u32, finish_type: EventFinishType) {
        let event_type = self.events.last().unwrap().event_type;
        let event_arg = self.events.last().unwrap().event_arg;

        self.player_data.target_actorid = ObjectTypeId::default();
        // sent event finish
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                handler_id,
                event_type,
                result: 1,
                arg: event_arg,
            });
            self.send_ipc_self(ipc).await;
        }

        // give back control to the player, or mark them as busy for some events
        match finish_type {
            EventFinishType::Normal => {
                self.conditions
                    .remove_condition(Condition::OccupiedInQuestEvent);
            }
            EventFinishType::Jumping => {
                // We want it set here, because when the client finishes the animation, they will send us a client trigger to tell us.
                self.conditions.set_condition(Condition::OccupiedInEvent);
            }
        };

        self.send_conditions().await;

        // Pop off the event stack
        self.events.pop();
    }

    /// Starts a new event. This can be nested, depending on the event type you chose. Returns true if the event was successfully found and scripted, otherwise flase.
    pub async fn start_event(
        &mut self,
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: EventType,
        event_arg: u32,
        lua_player: &mut LuaPlayer,
    ) -> bool {
        self.player_data.target_actorid = actor_id;

        // tell the client the event has started
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventStart(EventStart {
                target_id: actor_id,
                event_id,
                event_type,
                event_arg,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;

            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::DisableEventPosRollback { event_id },
            })
            .await;
        }

        // call into the event dispatcher, get the event
        let event;
        {
            let lua = self.lua.lock();

            event = lua
                .scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player)?;

                    let func: Function = lua.globals().get("dispatchEvent").unwrap();

                    func.call::<Option<Event>>((connection_data, event_id))
                })
                .unwrap();
        }

        if let Some(mut event) = event {
            event.event_type = event_type;
            event.event_arg = event_arg; // It turns out these same values HAVE to be sent in EventFinish, otherwise the game client crashes.
            self.events.push(event);

            true
        } else {
            let event_handler_type = event_id >> 16;

            tracing::warn!(
                "Event {event_id} ({}) isn't scripted yet! Ignoring...",
                EventHandlerType::from_repr(event_handler_type)
                    .map(|x| format!("{:?}", x))
                    .unwrap_or(format!("{event_handler_type}"))
            );

            // give control back to the player so they aren't stuck
            {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                    handler_id: event_id,
                    event_type,
                    result: 1,
                    arg: event_arg,
                });
                self.send_ipc_self(ipc).await;
            }

            self.send_notice(&format!(
                "Event {event_id} tried to start, but it doesn't have a script associated with it!"
            ))
            .await;

            false
        }
    }
}
