//! Handling all things related to the event system.

use mlua::Function;

use crate::{Event, ZoneConnection, lua::LuaPlayer};
use kawari::{
    common::{HandlerId, HandlerType, ObjectTypeId},
    config::get_config,
    ipc::zone::{
        ActorControlCategory, Condition, EventResume, EventScene, EventStart, EventType,
        SceneFlags, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    /// Starts a scene for the current event.
    pub async fn event_scene(
        &mut self,
        event_id: u32,
        scene: u16,
        mut scene_flags: SceneFlags,
        params: Vec<u32>,
        lua_player: &mut LuaPlayer,
    ) {
        let Some(event) = self.events.last() else {
            tracing::warn!("Tried to play scene with no event loaded?!");
            return;
        };

        let config = get_config();
        if config.tweaks.always_allow_skipping {
            scene_flags.set(SceneFlags::DISABLE_SKIP, false);
        }

        let scene = EventScene {
            actor_id: event.actor_id,
            handler_id: HandlerId(event_id),
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
            self.event_finish(event_id, lua_player).await;
        }
    }

    /// Finishes the current event, including resetting any conditions set during the start of said event.
    pub async fn event_finish(&mut self, handler_id: u32, lua_player: &mut LuaPlayer) {
        let event_type = self.events.last().unwrap().event_type;
        let event_arg = self.events.last().unwrap().event_arg;

        // sent event finish
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                handler_id: HandlerId(handler_id),
                event_type,
                result: 1,
                arg: event_arg,
            });
            self.send_ipc_self(ipc).await;
        }

        // Remove the condition given at the start of the event
        if let Some(condition) = self.events.last().unwrap().condition {
            self.conditions.remove_condition(condition);
        }

        // Despite that, we *always* have to send this otherwise the client gets stuck sometimes.
        self.send_conditions().await;

        // Pop off the event stack
        self.events.pop();

        if let Some(event) = self.events.last() {
            lua_player.event_handler_id = Some(HandlerId(event.id));
        } else {
            lua_player.event_handler_id = None;
        }
    }

    /// Starts a new event. This can be nested, depending on the event type you chose. Returns true if the event was successfully found and scripted, otherwise flase.
    pub async fn start_event(
        &mut self,
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: EventType,
        event_arg: u32,
        condition: Option<Condition>,
        lua_player: &mut LuaPlayer,
    ) -> bool {
        let old_event_handler_id = lua_player.event_handler_id;
        lua_player.event_handler_id = Some(HandlerId(event_id));

        // tell the client the event has started
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventStart(EventStart {
                target_id: actor_id,
                handler_id: HandlerId(event_id),
                event_type,
                event_arg,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;

            self.actor_control_self(ActorControlCategory::DisableEventPosRollback {
                handler_id: HandlerId(event_id),
            })
            .await;
        }

        // call into the event dispatcher, get the event
        let event;
        {
            let lua = self.lua.lock();

            event = lua
                .0
                .scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player)?;

                    let func: Function = lua.0.globals().get("dispatchEvent").unwrap();

                    func.call::<Option<Event>>((connection_data, event_id))
                })
                .unwrap();
        }

        if let Some(mut event) = event {
            event.event_type = event_type;
            event.event_arg = event_arg; // It turns out these same values HAVE to be sent in EventFinish, otherwise the game client crashes.
            event.condition = condition;
            event.actor_id = actor_id;
            self.events.push(event);

            true
        } else {
            let event_handler_type = event_id >> 16;

            tracing::warn!(
                "Event {event_id} ({}) isn't scripted yet! Ignoring...",
                HandlerType::from_repr(event_handler_type)
                    .map(|x| format!("{:?}", x))
                    .unwrap_or(format!("{event_handler_type}"))
            );

            // give control back to the player so they aren't stuck
            {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                    handler_id: HandlerId(event_id),
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

            lua_player.event_handler_id = old_event_handler_id;

            false
        }
    }

    /// Resumes the current event.
    pub async fn resume_event(&mut self, event_id: u32, scene: u16, params: Vec<u32>) {
        let scene = EventResume {
            handler_id: HandlerId(event_id),
            scene,
            params_count: params.len() as u8,
            params,
            unk2: 21, // TODO: lol what does this mean
        };
        if let Some(ipc) = scene.package_resume() {
            self.send_ipc_self(ipc).await;
        } else {
            tracing::error!("Unable to resume event {event_id}, scene {:?}!", scene);
        }
    }
}
