//! `io::Write` adapter that ships every byte to a zellij-server pane via
//! `Action::WriteToPaneId` over an `EmbeddedInputSender`.
//!
//! Slice 1b.3.c: stored inside `wezterm_term::Terminal` as its writer so
//! `Terminal::key_down` / `Terminal::mouse_event` encoded bytes flow to
//! zellij with zero glue per call site.

use std::io;

use zellij_server::embedded::EmbeddedInputSender;
use zellij_utils::data::PaneId as ZellijPaneId;
use zellij_utils::input::actions::Action;
use zellij_utils::ipc::ClientToServerMsg;

pub(crate) struct ZellijInputWriter {
    sender: EmbeddedInputSender,
    zellij_pane_id: u32,
    client_id: u16,
}

impl ZellijInputWriter {
    pub fn new(sender: EmbeddedInputSender, zellij_pane_id: u32, client_id: u16) -> Self {
        Self {
            sender,
            zellij_pane_id,
            client_id,
        }
    }
}

impl io::Write for ZellijInputWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let action = Action::WriteToPaneId {
            bytes: bytes.to_vec(),
            pane_id: ZellijPaneId::Terminal(self.zellij_pane_id),
        };
        let msg = ClientToServerMsg::Action {
            action,
            terminal_id: Some(self.zellij_pane_id),
            client_id: Some(self.client_id),
            is_cli_client: false,
        };
        self.sender
            .send(msg)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zellij_server::embedded::Server;
    use zellij_utils::ipc::IpcReceiverWithContext;

    /// Verify ZellijInputWriter::write packages bytes into Action::WriteToPaneId
    /// and the matching server-end IpcReceiver deserializes the same.
    #[test]
    fn writer_emits_write_to_pane_id_action() {
        // Build a Server (zero-side-effect: no thread spawned), borrow input
        // sender, install matching server-side receiver via direct socketpair
        // bypass — tests must NOT call start_session_blocking which would
        // need full plugin assets. We replicate Server::take_input_sender
        // round-trip pattern: use a fresh local SocketpairChannel.
        use zellij_server::embedded::SocketpairChannel;
        use zellij_utils::ipc::{ClientToServerMsg, IpcSenderWithContext};
        let _ = Server::new(); // sanity: type still constructs
        let (host, server) = SocketpairChannel::new().unwrap();
        let host_stream = host.into_local_socket_stream().unwrap();
        let server_stream = server.into_local_socket_stream().unwrap();

        let sender = EmbeddedInputSender::new_for_test(
            IpcSenderWithContext::<ClientToServerMsg>::new(host_stream),
        );
        let mut writer = ZellijInputWriter::new(sender, /* pane */ 7, /* client */ 1);
        writer.write_all(b"hello").unwrap();

        let mut recv = IpcReceiverWithContext::<ClientToServerMsg>::new(server_stream);
        let (msg, _ctx) = recv.recv_client_msg().expect("recv");
        match msg {
            ClientToServerMsg::Action {
                action: Action::WriteToPaneId { bytes, pane_id },
                terminal_id,
                client_id,
                is_cli_client,
            } => {
                assert_eq!(bytes, b"hello");
                assert!(matches!(pane_id, ZellijPaneId::Terminal(7)));
                assert_eq!(terminal_id, Some(7));
                assert_eq!(client_id, Some(1));
                assert!(!is_cli_client);
            }
            other => panic!("unexpected msg: {:?}", other),
        }
    }
}
