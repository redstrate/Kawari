use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    FromServer, ToServer,
    server::{DestinationNetwork, network::NetworkState},
};

/// Process linkshell-related messages.
pub fn handle_linkshell_messages(network: Arc<Mutex<NetworkState>>, msg: &ToServer) -> bool {
    match msg {
        ToServer::SetLinkshells(from_actor_id, linkshells) => {
            let mut network = network.lock();

            for linkshell_id in linkshells {
                // Just in case, skip this id if we're given a vec that contains entries with "no linkshell" (id 0).
                if *linkshell_id == 0 {
                    continue;
                }

                let shell = network.linkshells.entry(*linkshell_id).or_default();
                if !shell.contains(from_actor_id) {
                    shell.push(*from_actor_id);
                }
            }

            // Tell the chat connection it's time to refresh its info.
            let msg = FromServer::MustRefreshChatChannels();
            network.send_to_by_actor_id(*from_actor_id, msg, DestinationNetwork::ChatClients);

            true
        }
        ToServer::DisbandLinkshell(linkshell_id, linkshell_name) => {
            let mut network = network.lock();

            if !network.linkshells.contains_key(linkshell_id) {
                return true;
            };

            let msg = FromServer::LinkshellDisbanded(*linkshell_id, linkshell_name.clone());

            // Tell both zone and chat connections about the linkshell's disbandment first to avoid issues.
            network.send_to_linkshell(
                *linkshell_id,
                None,
                msg.clone(),
                DestinationNetwork::ZoneClients,
            );

            let msg = FromServer::MustRefreshChatChannels();
            network.send_to_linkshell(*linkshell_id, None, msg, DestinationNetwork::ChatClients);

            // Now update our state.
            network.linkshells.remove(linkshell_id);

            true
        }
        ToServer::LeaveLinkshell(
            target_actor_id,
            execute_content_id,
            target_content_id,
            from_name,
            reason_for_leaving,
            linkshell_id,
        ) => {
            let mut network = network.lock();

            {
                if !network.linkshells.contains_key(linkshell_id) {
                    return true;
                };

                let msg = FromServer::LinkshellLeft(
                    *target_actor_id,
                    *execute_content_id,
                    *target_content_id,
                    from_name.clone(),
                    *reason_for_leaving,
                    *linkshell_id,
                );

                // Tell both zone and chat connections about the member's departure first to avoid issues.
                network.send_to_linkshell(
                    *linkshell_id,
                    None,
                    msg.clone(),
                    DestinationNetwork::ZoneClients,
                );

                let msg = FromServer::MustRefreshChatChannels();
                network.send_to_by_actor_id(*target_actor_id, msg, DestinationNetwork::ChatClients);
            }

            // Now update our state by removing the leaving member, and removing the linkshell from our list if nobody is online.
            network
                .linkshells
                .entry(*linkshell_id)
                .and_modify(|ls| ls.retain(|m| *m != *target_actor_id));
            network.linkshells.retain(|_, shell| !shell.is_empty());
            true
        }
        ToServer::RenameLinkshell(from_content_id, from_name, linkshell_id, linkshell_name) => {
            let mut network = network.lock();

            let msg = FromServer::LinkshellRenamed(
                *from_content_id,
                from_name.clone(),
                *linkshell_id,
                linkshell_name.clone(),
            );

            network.send_to_linkshell(*linkshell_id, None, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::SetLinkshellRank(
            linkshell_id,
            from_content_id,
            target_content_id,
            rank,
            target_name,
        ) => {
            let mut network = network.lock();

            let msg = FromServer::LinkshellRankChanged(
                *linkshell_id,
                *from_content_id,
                *target_content_id,
                *rank,
                target_name.clone(),
            );

            network.send_to_linkshell(*linkshell_id, None, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::SendLinkshellInvite(target_actor_id, invite_info) => {
            let mut network = network.lock();
            {
                let Some(linkshell) = network.linkshells.get_mut(&invite_info.linkshell_id) else {
                    return true;
                };

                if !linkshell.contains(target_actor_id) {
                    linkshell.push(*target_actor_id);
                }
            }

            let msg = FromServer::LinkshellInviteReceived(invite_info.clone());
            network.send_to_by_actor_id(*target_actor_id, msg, DestinationNetwork::ZoneClients);

            // Refresh due to now being part of this LS. The client does block chat messages as an invitee, but we can do better and just not send them the packets at all.
            let msg = FromServer::MustRefreshChatChannels();
            network.send_to_by_actor_id(*target_actor_id, msg, DestinationNetwork::ChatClients);

            true
        }
        ToServer::AcceptedLinkshellInvite(
            from_actor_id,
            linkshell_id,
            from_content_id,
            from_name,
            linkshell_name,
        ) => {
            let mut network = network.lock();

            {
                let Some(linkshell) = network.linkshells.get_mut(linkshell_id) else {
                    return true;
                };

                if !linkshell.contains(from_actor_id) {
                    linkshell.push(*from_actor_id);
                }
            }
            let msg = FromServer::LinkshellInviteAccepted(
                *linkshell_id,
                *from_content_id,
                from_name.clone(),
                linkshell_name.clone(),
            );

            network.send_to_linkshell(*linkshell_id, None, msg, DestinationNetwork::ZoneClients);

            // Refresh due to member rank changing from Invitee to Member
            let msg = FromServer::MustRefreshChatChannels();
            network.send_to_by_actor_id(*from_actor_id, msg, DestinationNetwork::ChatClients);

            true
        }
        _ => false,
    }
}
