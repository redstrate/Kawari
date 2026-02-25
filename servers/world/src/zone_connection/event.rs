//! Handling all things related to the event system.

use crate::{
    Event, ZoneConnection,
    event::{EventHandler, dispatch_event},
};
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
        event: &Event,
        scene: u16,
        mut scene_flags: SceneFlags,
        params: Vec<u32>,
    ) {
        let config = get_config();
        if config.tweaks.always_allow_skipping {
            scene_flags.set(SceneFlags::DISABLE_SKIP, false);
        }

        let scene = EventScene {
            actor_id: event.actor_id,
            handler_id: HandlerId(event.id),
            scene,
            scene_flags,
            params_count: params.len() as u8,
            params,
            ..Default::default()
        };
        if let Some(ipc) = scene.package() {
            self.send_ipc_self(ipc).await;
        } else {
            tracing::error!(
                "Unable to play event {}, scene {:?}, scene_flags {scene_flags}!",
                event.id,
                scene
            );
        }
    }

    /// Finishes the current event, including resetting any conditions set during the start of said event.
    pub async fn event_finish(&mut self, events: &mut Vec<(Box<dyn EventHandler>, Event)>) {
        if let Some(event) = events.pop() {
            let event_type = event.1.event_type;
            let event_arg = event.1.event_arg;
            let event_id = event.1.id;

            // sent event finish
            {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                    handler_id: HandlerId(event_id),
                    event_type,
                    result: 1,
                    arg: event_arg,
                });
                self.send_ipc_self(ipc).await;
            }

            // Remove the condition given at the start of the event
            if let Some(condition) = event.1.condition {
                self.conditions.remove_condition(condition);
            }

            // Despite that, we *always* have to send this otherwise the client gets stuck sometimes.
            self.send_conditions().await;
        }

        if let Some(event) = events.last() {
            self.event_handler_id = Some(HandlerId(event.1.id));
        } else {
            self.event_handler_id = None;
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
        events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    ) -> bool {
        let old_event_handler_id = self.event_handler_id;
        self.event_handler_id = Some(HandlerId(event_id));

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
        let handler = dispatch_event(HandlerId(event_id), self.gamedata.clone());

        if let Some(handler) = handler {
            events.push((
                handler,
                Event {
                    id: event_id,
                    event_type,
                    event_arg,
                    condition,
                    actor_id,
                },
            ));

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

            self.event_handler_id = old_event_handler_id;

            false
        }
    }

    /// Resumes the current event.
    pub async fn resume_event(
        &mut self,
        event_id: u32,
        scene: u16,
        resume_id: u8,
        params: Vec<u32>,
    ) {
        let scene = EventResume {
            handler_id: HandlerId(event_id),
            scene,
            resume_id,
            params_count: params.len() as u8,
            params,
        };
        if let Some(ipc) = scene.package() {
            self.send_ipc_self(ipc).await;
        } else {
            tracing::error!("Unable to resume event {event_id}, scene {:?}!", scene);
        }
    }
}
