#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Filter(String),
    Save(String),
    Clear,
    Tick,
    Quit,
    NavigateToHome,
    NavigateToDevice,
    NavigateToSniffer,
    DeviceSelected(String),
    ApplyFilter(String),
    Handled,
    PacketSelected(usize), // New action for packet selection
}
