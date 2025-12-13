mod handle;
pub use handle::SessionHandle;

pub type SessionID = u8;

#[derive(Debug, Default)]
pub struct SessionManager {
    handles: Vec<SessionHandle>,
    metas: Vec<SessionMeta>,
    active: Option<SessionID>,
}

/// Includes the baud rate and name of the port/connection.
#[derive(Debug, Clone)]
pub struct SessionMeta {
    pub(crate) baud: u32,
    pub(crate) port: String,
}

impl SessionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: None,
            // Creating with capacity of ~25 because if someone has more than 25
            // sessions they can get an allocation - majority will have < 25 sessions
            handles: Vec::with_capacity((u8::MAX / 10u8) as usize),
            metas: Vec::with_capacity((u8::MAX / 10u8) as usize),
        }
    }

    /// Get a reference to the [`SessionHandle`] of [`SessionID`].
    ///
    /// Returns `None` if the [`SessionID`] is invalid.
    #[must_use]
    pub fn get_session(&self, id: SessionID) -> Option<&SessionHandle> {
        self.handles.get(id as usize)
    }

    /// Get a reference to the [`SessionMeta`] of [`SessionID`].
    ///
    /// Returns `None` if the [`SessionID`] is invalid.
    #[must_use]
    pub fn get_meta(&self, id: SessionID) -> Option<&SessionMeta> {
        self.metas.get(id as usize)
    }

    /// Create a session at `port` with the specified `baud` rate.
    ///
    /// # Errors
    /// Errors if there are already a maximum number of sessions ([`u8::MAX`]) or
    /// if the [`SessionHandle::spawn`] errors.
    /// TODO: HANDLE ERROR PROPAGATING TO STDOUT
    #[allow(clippy::result_unit_err)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn spawn(&mut self, port: &str, baud: u32) -> miette::Result<SessionID> {
        let id = self.metas.len();

        if !id < u8::MAX as usize {
            tracing::warn!(target: "session::spawn", %id, "max sessions reached");
            return Err(()).map_err(|_| miette::miette!("Max sessions reached"))?; // TODO: HANDLE STDOUT ERROR PROPAGATING
        }

        let meta = SessionMeta {
            baud,
            port: port.to_owned(),
        };
        let handle = SessionHandle::spawn(&meta)?;

        self.handles.push(handle);
        self.metas.push(meta);

        // Cast is fine, verified that id is < u8::MAX
        Ok(id as SessionID)
    }

    /// Kill/shutdown the session for [`SessionID`]
    #[allow(clippy::cast_possible_truncation)]
    pub async fn kill(&mut self, id: SessionID) {
        let idx = id as usize;
        if idx >= self.metas.len() {
            return;
        }

        if self.handles.get(idx).is_none() {
            tracing::debug!(target: "session::kill", "session {id} does not exist");
            return;
        }

        self.metas.swap_remove(idx);
        self.handles.swap_remove(idx).shutdown().await;
        tracing::trace!(target: "session::kill", "session {idx} killed");

        // NOTE:
        // Shouldn't need to adjust active idx because when the user is in the
        // REPL, there should not be an active session; and this should only
        // be possible to call from the REPL. Check anyways in the off-chance
        // that this is not the case.
        debug_assert!(self.active.is_none());
        if let Some(active) = self.active {
            let a_idx = active as usize;
            if a_idx == idx {
                self.active = None;
            } else if a_idx == self.metas.len() {
                // Just moved last item into idx, renumber active to maintain
                // Cast is fine, verified that id is < u8::MAX
                self.active = Some(idx as SessionID);
            }
        }
    }

    /// List the sessions stored in [`SessionManager`], writes to the given [`Writer`].
    ///
    /// Writes in the following format:
    /// ```txt
    /// ID  PORT           BAUD
    /// 1   /dev/ttyUSB1   9600
    /// 2   /dev/ttyUSB2   115200
    /// ```
    ///
    /// [`Writer`]: std::io::Write
    pub fn list<W: std::io::Write>(&self, writer: &mut W) {
        #[cfg(target_os = "windows")]
        {
            let _ = writeln!(writer, "{:3} {:6} BAUD", "ID", "PORT");
            for (id, meta) in self.metas.iter().enumerate() {
                let _ = writeln!(writer, "{:<3} {:6} {}", id, meta.port, meta.baud);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = writeln!(writer, "{:3} {:14} BAUD", "ID", "PORT");
            for (id, meta) in self.metas.iter().enumerate() {
                let _ = writeln!(writer, "{:<3} {:14} {}", id, meta.port, meta.baud);
            }
        }
    }

    pub async fn graceful_shutdown(self) {
        let mut idx: u8 = 0;
        for session in self.handles {
            idx += 1;
            session.shutdown().await;
            tracing::info!(
                target: "session",
                port = %self.metas[(idx as usize).saturating_sub(1)].port,
                "shutdown session: '{idx}'"
            );
        }
    }
}
