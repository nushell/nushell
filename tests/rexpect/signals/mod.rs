mod interactive;
mod noninteractive;

/// Nu command to run an external command that sleeps but can be interrupted via a signal.
const EXTERNAL_SLEEP: &str = "bash --norc --noprofile -c 'read -t 1'";

/// Built in nu sleep command.
const INTERNAL_SLEEP: &str = "sleep 1sec";
