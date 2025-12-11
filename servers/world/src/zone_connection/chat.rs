//! Chat and command handling.

use mlua::Function;

use crate::{
    MessageInfo, ZoneConnection,
    lua::{ExtraLuaState, LuaPlayer},
};
use kawari::{
    config::get_config,
    ipc::zone::{
        ChatMessage, ServerNoticeFlags, ServerNoticeMessage, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    pub async fn send_message(&mut self, message: MessageInfo) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ChatMessage(ChatMessage {
            sender_account_id: message.sender_account_id,
            sender_actor_id: message.sender_actor_id,
            sender_world_id: message.sender_world_id,
            sender_name: message.sender_name,
            channel: message.channel,
            message: message.message,
            ..Default::default()
        }));
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_notice(&mut self, message: &str) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
            ServerNoticeMessage {
                flags: ServerNoticeFlags::CHAT_LOG,
                message: message.to_string(),
            },
        ));

        self.send_ipc_self(ipc).await;
    }

    pub async fn run_gm_command(
        &mut self,
        command: u32,
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
        lua_player: &mut LuaPlayer,
    ) {
        let lua = self.lua.lock();
        let state = lua.app_data_ref::<ExtraLuaState>().unwrap();
        let config = get_config();

        if let Some(command_script) = state.gm_command_scripts.get(&command) {
            let file_name = format!("{}/{}", &config.world.scripts_location, command_script);

            let mut run_script = || -> mlua::Result<()> {
                lua.scope(|scope| {
                    let connection_data = scope
                    .create_userdata_ref_mut(lua_player)?;
                    /* TODO: Instead of panicking we ought to send a message to the player
                     * and the console log, and abandon execution. */
                    lua.load(
                        std::fs::read(&file_name).unwrap_or_else(|_| panic!("Failed to load script file {}!", &file_name)),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()?;

                    let required_rank = lua.globals().get("required_rank");
                    if let Err(error) = required_rank {
                        tracing::info!("Script is missing required_rank! Unable to run command, sending error to user. Additional information: {}", error);
                        let func: Function =
                        lua.globals().get("onCommandRequiredRankMissingError")?;
                        func.call::<()>((error.to_string(), connection_data))?;
                        return Ok(());
                    }

                    /* Reset state for future commands. Without this it'll stay set to the last value
                     * and allow other commands that omit required_rank to run, which is undesirable. */
                    lua.globals().set("required_rank", mlua::Value::Nil)?;

                    if self.player_data.gm_rank as u8 >= required_rank? {
                        let func: Function =
                        lua.globals().get("onCommand")?;
                        func.call::<()>(([arg0, arg1, arg2, arg3], connection_data))?;

                        /* `command_sender` is an optional variable scripts can define to identify themselves in print messages.
                         * It's okay if this global isn't set. We also don't care what its value is, just that it exists.
                         * This is reset -after- running the command intentionally. Resetting beforehand will never display the command's identifier.
                         */
                        let command_sender: Result<mlua::prelude::LuaValue, mlua::prelude::LuaError> = lua.globals().get("command_sender");
                        if command_sender.is_ok() {
                            lua.globals().set("command_sender", mlua::Value::Nil)?;
                        }
                        Ok(())
                    } else {
                        tracing::info!("User with account_id {} tried to invoke GM command {} with insufficient privileges!",
                                       self.player_data.account_id, command);
                        let func: Function =
                        lua.globals().get("onCommandRequiredRankInsufficientError")?;
                        func.call::<()>(connection_data)?;
                        Ok(())
                    }
                })
            };

            if let Err(err) = run_script() {
                tracing::warn!("Lua error in {file_name}: {:?}", err);
            }
        } else {
            tracing::warn!(
                "Received unknown GM command {command} with args: arg0: {arg0} arg1: {arg1} arg2: {arg2} arg3: {arg3}!"
            );
        }
    }
}
