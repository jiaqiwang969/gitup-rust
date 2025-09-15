use crossterm::event::KeyEvent;
use std::collections::HashMap;

/// Macro recording and playback
pub struct MacroRecorder {
    // Currently recording register (None if not recording)
    recording: Option<char>,

    // Current macro being recorded
    current_macro: Vec<KeyEvent>,

    // Stored macros (register -> key sequence)
    macros: HashMap<char, Vec<KeyEvent>>,

    // Last executed macro for @@ repeat
    last_executed: Option<char>,
}

impl MacroRecorder {
    pub fn new() -> Self {
        Self {
            recording: None,
            current_macro: Vec::new(),
            macros: HashMap::new(),
            last_executed: None,
        }
    }

    /// Start recording a macro to a register
    pub fn start_recording(&mut self, register: char) -> Result<(), String> {
        if self.recording.is_some() {
            return Err("Already recording a macro".to_string());
        }

        if !register.is_ascii_alphanumeric() {
            return Err(format!("Invalid register: {}", register));
        }

        self.recording = Some(register);
        self.current_macro.clear();
        Ok(())
    }

    /// Stop recording the current macro
    pub fn stop_recording(&mut self) -> Option<char> {
        if let Some(register) = self.recording {
            // Store the macro
            self.macros.insert(register, self.current_macro.clone());
            self.recording = None;
            self.current_macro.clear();
            Some(register)
        } else {
            None
        }
    }

    /// Record a key event (if recording)
    pub fn record_key(&mut self, key: KeyEvent) {
        if self.recording.is_some() {
            self.current_macro.push(key);
        }
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Get the register being recorded to
    pub fn recording_register(&self) -> Option<char> {
        self.recording
    }

    /// Execute a macro from a register
    pub fn execute_macro(&mut self, register: char, count: usize) -> Option<Vec<KeyEvent>> {
        let register = if register == '@' {
            // @@ repeats last macro
            self.last_executed?
        } else {
            register
        };

        if let Some(macro_keys) = self.macros.get(&register) {
            self.last_executed = Some(register);

            // Repeat the macro 'count' times
            let mut result = Vec::new();
            for _ in 0..count {
                result.extend(macro_keys.clone());
            }
            Some(result)
        } else {
            None
        }
    }

    /// Append to an existing macro (uppercase register)
    pub fn append_to_macro(&mut self, register: char) -> Result<(), String> {
        let lower = register.to_ascii_lowercase();

        if self.recording.is_some() {
            return Err("Cannot append while recording".to_string());
        }

        // Get existing macro
        let existing = self.macros.get(&lower).cloned().unwrap_or_default();

        // Start recording with existing content
        self.recording = Some(lower);
        self.current_macro = existing;

        Ok(())
    }

    /// Clear a macro
    pub fn clear_macro(&mut self, register: char) {
        self.macros.remove(&register);
        if self.last_executed == Some(register) {
            self.last_executed = None;
        }
    }

    /// Clear all macros
    pub fn clear_all_macros(&mut self) {
        self.macros.clear();
        self.last_executed = None;
    }

    /// Get a macro for display/editing
    pub fn get_macro(&self, register: char) -> Option<&Vec<KeyEvent>> {
        self.macros.get(&register)
    }

    /// Set a macro directly (for loading from config)
    pub fn set_macro(&mut self, register: char, keys: Vec<KeyEvent>) {
        self.macros.insert(register, keys);
    }

    /// List all macros
    pub fn list_macros(&self) -> Vec<(char, usize)> {
        let mut macros: Vec<_> = self.macros
            .iter()
            .map(|(&reg, keys)| (reg, keys.len()))
            .collect();
        macros.sort_by_key(|(reg, _)| *reg);
        macros
    }

    /// Get status line indicator
    pub fn status_indicator(&self) -> Option<String> {
        self.recording.map(|reg| format!("recording @{}", reg))
    }
}