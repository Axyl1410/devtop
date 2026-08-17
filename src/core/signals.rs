#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProcessSignal {
    Term, // SIGTERM (15) - Graceful termination
    Kill, // SIGKILL (9)  - Immediate hard kill
    Int,  // SIGINT (2)   - Interrupt from keyboard
    Hup,  // SIGHUP (1)   - Hangup / Reload configuration
    Stop, // SIGSTOP (19) - Pause process
    Cont, // SIGCONT (18) - Resume paused process
}

impl ProcessSignal {
    pub const ALL: &'static [ProcessSignal] = &[
        Self::Term, // 15
        Self::Kill, // 9
        Self::Int,  // 2
        Self::Hup,  // 1
        Self::Stop, // 19
        Self::Cont, // 18
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Term => "15: SIGTERM",
            Self::Kill => " 9: SIGKILL",
            Self::Int => " 2: SIGINT",
            Self::Hup => " 1: SIGHUP",
            Self::Stop => "19: SIGSTOP",
            Self::Cont => "18: SIGCONT",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Term => "Graceful termination (Recommended / Default)",
            Self::Kill => "Force kill immediately (Hard uncatchable stop)",
            Self::Int => "Interrupt from terminal (Ctrl+C equivalent)",
            Self::Hup => "Hangup / Reload configuration",
            Self::Stop => "Suspend / Pause execution",
            Self::Cont => "Resume suspended execution",
        }
    }

    pub fn is_dangerous(&self) -> bool {
        matches!(self, Self::Kill)
    }

    #[cfg(unix)]
    pub fn to_raw_signal(&self) -> libc::c_int {
        match self {
            Self::Term => libc::SIGTERM,
            Self::Kill => libc::SIGKILL,
            Self::Int => libc::SIGINT,
            Self::Hup => libc::SIGHUP,
            Self::Stop => libc::SIGSTOP,
            Self::Cont => libc::SIGCONT,
        }
    }
}

pub fn send_signal_to_pid(pid: u32, signal: ProcessSignal) -> Result<(), String> {
    #[cfg(unix)]
    {
        let sig = signal.to_raw_signal();
        let ret = unsafe { libc::kill(pid as libc::pid_t, sig) };
        if ret == 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            Err(format!("Failed to send {}: {}", signal.name(), err))
        }
    }
    #[cfg(not(unix))]
    {
        // On non-unix fallback, only Kill is supported
        Err("Signal sending is only supported on Unix-like systems".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_signals_properties() {
        assert_eq!(ProcessSignal::ALL.len(), 6);
        assert_eq!(ProcessSignal::ALL[0], ProcessSignal::Term);
        assert!(!ProcessSignal::Term.is_dangerous());
        assert!(ProcessSignal::Kill.is_dangerous());

        assert!(ProcessSignal::Term.name().contains("15"));
        assert!(ProcessSignal::Kill.name().contains("9"));
        assert!(!ProcessSignal::Term.description().is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn test_raw_signal_mapping() {
        assert_eq!(ProcessSignal::Term.to_raw_signal(), libc::SIGTERM);
        assert_eq!(ProcessSignal::Kill.to_raw_signal(), libc::SIGKILL);
        assert_eq!(ProcessSignal::Int.to_raw_signal(), libc::SIGINT);
        assert_eq!(ProcessSignal::Hup.to_raw_signal(), libc::SIGHUP);
        assert_eq!(ProcessSignal::Stop.to_raw_signal(), libc::SIGSTOP);
        assert_eq!(ProcessSignal::Cont.to_raw_signal(), libc::SIGCONT);
    }

    #[test]
    #[cfg(unix)]
    fn test_send_signal_nonexistent_pid() {
        // PID 4194304 is beyond standard Linux max PID -> should fail with error
        let res = send_signal_to_pid(4_000_000, ProcessSignal::Term);
        assert!(res.is_err());
    }
}
